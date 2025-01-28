#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod windows {
    use std::{
        ffi::OsStr,
        fs::File,
        io::{self, Read},
        os::windows::{
            ffi::OsStrExt,
            process::CommandExt,
        },
        path::PathBuf,
        process::Command,
        sync::{Arc, Mutex},
        thread,
        time::{Duration, Instant},
    };
    use lazy_static::lazy_static;
    use regex::Regex;
    use winapi::{
        shared::minwindef::{BYTE, DWORD},
        um::{
            errhandlingapi::GetLastError,
            winbase::CREATE_NO_WINDOW,
            winnt::LPWSTR,
            winspool::{
                ClosePrinter, EndDocPrinter, EndPagePrinter, OpenPrinterW, StartDocPrinterW,
                StartPagePrinter, WritePrinter, DOC_INFO_1W,
            },
        },
    };

    lazy_static! {
        static ref PRINT_QUEUE: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
    }

    const FLAG_TIMEOUT: Duration = Duration::from_secs(300);
    const PRINT_WAIT_INTERVAL: Duration = Duration::from_millis(100);

    struct PrinterHandle {
        // handle: winapi::um::winspool::HANDLE,
        handle: winapi::shared::ntdef::HANDLE
    }

    impl PrinterHandle {
        fn open(printer_name: &str) -> io::Result<Self> {
            let name_wide: Vec<u16> = OsStr::new(printer_name)
                .encode_wide()
                .chain(Some(0))
                .collect();
            let mut handle = std::ptr::null_mut();
            
            unsafe {
                if OpenPrinterW(name_wide.as_ptr() as LPWSTR, &mut handle, std::ptr::null_mut()) == 0 {
                    let err = GetLastError();
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Failed to open printer (Error {})", err),
                    ));
                }
            }
            
            Ok(Self { handle })
        }
    }

    impl Drop for PrinterHandle {
        fn drop(&mut self) {
            unsafe { ClosePrinter(self.handle) };
        }
    }

    fn get_file_extension(file_path: &str) -> Option<String> {
        Regex::new(r"(?i)\.([a-zA-Z0-9]+)$")
            .unwrap()
            .captures(file_path)
            .map(|cap| cap[1].to_lowercase())
    }

    fn read_config(config_file_path: &str) -> io::Result<Vec<(Regex, String)>> {
        let config = std::fs::read_to_string(config_file_path)?;
        let result = config
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, '|').collect();
                if parts.len() != 2 {
                    return None;
                }
                Regex::new(&format!("(?i){}", parts[0]))
                    .ok()
                    .map(|re| (re, parts[1].trim().to_string()))
            })
            .collect::<Vec<_>>();
        Ok(result)
    }

    fn send_raw_to_printer(printer_name: &str, file_path: &str) -> io::Result<()> {
        let printer = PrinterHandle::open(printer_name)?;
        let mut file = File::open(file_path)?;
        let doc_name: Vec<u16> = OsStr::new("RAW Print Job")
            .encode_wide()
            .chain(Some(0))
            .collect();
        let datatype: Vec<u16> = OsStr::new("RAW")
            .encode_wide()
            .chain(Some(0))
            .collect();

        unsafe {
            let doc_info = DOC_INFO_1W {
                pDocName: doc_name.as_ptr() as LPWSTR,
                pOutputFile: std::ptr::null_mut(),
                pDatatype: datatype.as_ptr() as LPWSTR,
            };

            let job_id = StartDocPrinterW(printer.handle, 1, &doc_info as *const _ as *mut BYTE);
            if job_id <= 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("StartDocPrinter failed (Error {})", GetLastError()),
                ));
            }

            if StartPagePrinter(printer.handle) == 0 {
                EndDocPrinter(printer.handle);
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("StartPagePrinter failed (Error {})", GetLastError()),
                ));
            }

            let mut buffer = [0u8; 4096];
            loop {
                let bytes_read = match file.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(e) => {
                        EndPagePrinter(printer.handle);
                        EndDocPrinter(printer.handle);
                        return Err(e);
                    }
                };

                let mut bytes_written = 0;
                let mut remaining = bytes_read;
                let mut offset = 0;

                while remaining > 0 {
                    if WritePrinter(
                        printer.handle,
                        buffer[offset..].as_ptr() as *mut _,
                        remaining as DWORD,
                        &mut bytes_written,
                    ) == 0
                    {
                        let err = GetLastError();
                        EndPagePrinter(printer.handle);
                        EndDocPrinter(printer.handle);
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            format!("WritePrinter failed (Error {})", err),
                        ));
                    }

                    remaining -= bytes_written as usize;
                    offset += bytes_written as usize;
                }
            }

            if EndPagePrinter(printer.handle) == 0 || EndDocPrinter(printer.handle) == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to end print job (Error {})", GetLastError()),
                ));
            }
        }

        Ok(())
    }

    fn print_pdf(printer: &str, file_path: &str) -> io::Result<()> {
        let current_exe = std::env::current_exe()?;
        let exe_dir = current_exe.parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No parent directory"))?;
        let pdftoprinter = exe_dir.join("PDFtoPrinter.exe");
        
        if !pdftoprinter.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "PDFtoPrinter.exe not found",
            ));
        }

        let start = Instant::now();
        while is_printing_in_progress()? {
            if start.elapsed() > FLAG_TIMEOUT {
                clean_stale_flag()?;
                break;
            }
            thread::sleep(PRINT_WAIT_INTERVAL);
        }

        create_flag_file()?;
        let status = Command::new(pdftoprinter)
            .args(&[file_path, printer.trim_matches('\\')])
            .creation_flags(CREATE_NO_WINDOW)
            .status()?;
        delete_flag_file()?;

        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("PDFtoPrinter failed with exit code: {:?}", status.code()),
            ));
        }

        Ok(())
    }

    fn get_flag_path() -> io::Result<PathBuf> {
        let current_exe = std::env::current_exe()?;
        Ok(current_exe.parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No parent directory"))?
            .join("printing.flag"))
    }

    fn create_flag_file() -> io::Result<()> {
        File::create(get_flag_path()?)?;
        Ok(())
    }

    fn delete_flag_file() -> io::Result<()> {
        let flag = get_flag_path()?;
        if flag.exists() {
            std::fs::remove_file(flag)?;
        }
        Ok(())
    }

    fn is_printing_in_progress() -> io::Result<bool> {
        let flag = get_flag_path()?;
        if !flag.exists() {
            return Ok(false);
        }

        let metadata = std::fs::metadata(&flag)?;
        if let Ok(modified) = metadata.modified() {
            if modified.elapsed().unwrap_or(Duration::MAX) > FLAG_TIMEOUT {
                clean_stale_flag()?;
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn clean_stale_flag() -> io::Result<()> {
        let flag = get_flag_path()?;
        if flag.exists() {
            std::fs::remove_file(flag)?;
        }
        Ok(())
    }

    fn process_queue() {
        let mut queue = PRINT_QUEUE.lock().unwrap();
        while let Some((printer, file)) = queue.pop() {
            let result = if get_file_extension(&file).unwrap_or_default() == "pdf" {
                print_pdf(&printer, &file)
            } else {
                send_raw_to_printer(&printer, &file)
            };

            if let Err(e) = result {
                eprintln!("Failed to print {}: {}", file, e);
            }
        }
    }

    pub fn main() -> io::Result<()> {
        let args: Vec<String> = std::env::args().collect();
        if args.len() < 2 {
            eprintln!("Usage: {} <file-path>", args[0]);
            return Ok(());
        }

        let file_path = &args[1];
        let current_exe = std::env::current_exe()?;
        let exe_dir = current_exe.parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No parent directory"))?;
        let config_path = exe_dir.join("config.txt");

        let mappings = read_config(config_path.to_str().unwrap())?;
        let target_printer = mappings
            .into_iter()
            .find(|(re, _)| re.is_match(file_path))
            .map(|(_, printer)| printer);

        if let Some(printer) = target_printer {
            PRINT_QUEUE.lock().unwrap().push((printer, file_path.to_string()));
            process_queue();
        } else {
            eprintln!("No printer mapping found for {}", file_path);
        }

        Ok(())
    }
}

#[cfg(not(windows))]
fn main() {
    panic!("This application is only supported on Windows");
}

#[cfg(windows)]
fn main() -> std::io::Result<()> {
    windows::main()
}
