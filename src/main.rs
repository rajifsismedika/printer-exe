use std::{
    ffi::OsStr,
    fs::File,
    io::{self, Read},
    os::windows::{
        ffi::OsStrExt,
        process::CommandExt,
    },
    ptr::null_mut,
    process::Command,
    sync::Mutex,
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

// Global queue for print jobs
lazy_static! {
    static ref PRINT_QUEUE: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
}

/// Gets the file extension from a file path.
fn get_file_extension(file_path: &str) -> Option<String> {
    Regex::new(r"\.([a-zA-Z0-9]+)$")
        .unwrap()
        .captures(file_path)
        .map(|cap| cap[1].to_string())
}

/// Reads the configuration file and returns a mapping of file extensions to printer names.
fn read_config(config_file_path: &str) -> io::Result<Vec<(Regex, String)>> {
    let config_file = std::fs::read_to_string(config_file_path)?;
    Ok(config_file
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() == 2 {
                Regex::new(parts[0])
                    .map(|re| (re, parts[1].to_string()))
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
                    .ok()
            } else {
                None
            }
        })
        .collect())
}

/// Sends a raw print job to the specified printer.
fn send_print_raw_job(printer_name: &str, document_path: &str) -> io::Result<()> {
    let printer_name_wide: Vec<u16> = OsStr::new(printer_name).encode_wide().chain(Some(0)).collect();
    let mut h_printer = null_mut();

    unsafe {
        if OpenPrinterW(printer_name_wide.as_ptr() as LPWSTR, &mut h_printer, null_mut()) == 0 {
            let error = GetLastError();
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to open printer. Error: {}", error),
            ));
        }

        let mut file = File::open(document_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let doc_name: Vec<u16> = OsStr::new("Printing File").encode_wide().chain(Some(0)).collect();
        let raw_datatype: Vec<u16> = OsStr::new("RAW").encode_wide().chain(Some(0)).collect();
        let doc_info = DOC_INFO_1W {
            pDocName: doc_name.as_ptr() as LPWSTR,
            pOutputFile: null_mut(),
            pDatatype: raw_datatype.as_ptr() as LPWSTR,
        };

        let job_id = StartDocPrinterW(h_printer, 1, &doc_info as *const _ as *mut BYTE);
        if job_id <= 0 {
            let error = GetLastError();
            ClosePrinter(h_printer);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to start print job. Error: {}", error),
            ));
        }

        if StartPagePrinter(h_printer) == 0 {
            let error = GetLastError();
            EndDocPrinter(h_printer);
            ClosePrinter(h_printer);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to start page. Error: {}", error),
            ));
        }

        let mut bytes_written: DWORD = 0;
        if WritePrinter(h_printer, data.as_ptr() as *mut _, data.len() as DWORD, &mut bytes_written) == 0 {
            let error = GetLastError();
            EndPagePrinter(h_printer);
            EndDocPrinter(h_printer);
            ClosePrinter(h_printer);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to write to printer. Error: {}", error),
            ));
        }

        if EndPagePrinter(h_printer) == 0 || EndDocPrinter(h_printer) == 0 {
            let error = GetLastError();
            ClosePrinter(h_printer);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to end print job. Error: {}", error),
            ));
        }

        ClosePrinter(h_printer);
    }

    Ok(())
}

/// Returns the path to the flag file.
fn get_flag_file_path() -> io::Result<std::path::PathBuf> {
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get executable directory"))?;
    Ok(exe_dir.join("printing.flag"))
}

/// Creates the flag file to indicate that printing is in progress.
fn create_flag_file() -> io::Result<()> {
    File::create(get_flag_file_path()?)?;
    Ok(())
}

/// Deletes the flag file to indicate that printing is done.
fn delete_flag_file() -> io::Result<()> {
    let flag_file_path = get_flag_file_path()?;
    if flag_file_path.exists() {
        std::fs::remove_file(flag_file_path)?;
    }
    Ok(())
}

/// Checks if the flag file exists, indicating that printing is in progress.
fn is_printing_in_progress() -> io::Result<bool> {
    Ok(get_flag_file_path()?.exists())
}

/// Prints a file using the appropriate method based on its extension.
fn send_print_job(printer_name: &str, document_path: &str) -> io::Result<()> {
    if get_file_extension(document_path).unwrap_or_default() == "pdf" {
        let trimmed_printer_name = printer_name.trim_matches('\\');
        let exe_path = std::env::current_exe()?;
        let exe_dir = exe_path.parent().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get executable directory"))?;
        let pdftoprinter_path = exe_dir.join("PDFtoPrinter.exe");

        while is_printing_in_progress()? {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        create_flag_file()?;

        let status = Command::new(pdftoprinter_path)
            .args(&[document_path, trimmed_printer_name])
            .creation_flags(CREATE_NO_WINDOW)
            .status()?;

        delete_flag_file()?;

        if !status.success() {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to execute PDFtoPrinter.exe"));
        }
    } else {
        send_print_raw_job(printer_name, document_path)?;
    }

    Ok(())
}

/// Processes the print queue.
fn process_print_queue() {
    let mut queue = PRINT_QUEUE.lock().unwrap();
    while let Some((printer_name, document_path)) = queue.pop() {
        if let Err(e) = send_print_job(&printer_name, &document_path) {
            eprintln!("Failed to print {}: {}", document_path, e);
        }
    }
}

/// Adds a print job to the queue.
fn add_print_job(printer_name: String, document_path: String) {
    PRINT_QUEUE.lock().unwrap().push((printer_name, document_path));
}

/// Ensures the flag file is deleted when the program exits.
struct FlagFileCleanup;

impl Drop for FlagFileCleanup {
    fn drop(&mut self) {
        let _ = delete_flag_file();
    }
}

fn main() -> io::Result<()> {
    let _cleanup = FlagFileCleanup;

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        return Ok(());
    }

    let file_path = &args[1];
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get executable directory"))?;
    let config_file_path = exe_dir.join("config.txt");

    let mappings = read_config(config_file_path.to_str().unwrap())?;
    let file_extension = get_file_extension(file_path).unwrap_or_default();

    if let Some(printer_name) = mappings
        .into_iter()
        .find(|(re, _)| re.is_match(file_path))
        .map(|(_, printer_name)| printer_name)
    {
        add_print_job(printer_name, file_path.to_string());
        process_print_queue();
    } else {
        eprintln!("No printer found for file extension: {}", file_extension);
    }

    Ok(())
}
