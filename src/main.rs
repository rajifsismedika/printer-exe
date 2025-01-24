use std::fs::File;
use std::io::{self, Read};
use regex::Regex;

/// Gets the file extension from a file path.
fn get_file_extension(file_path: &str) -> Option<String> {
    let re = Regex::new(r"\.([a-zA-Z0-9]+)$").unwrap();
    re.captures(file_path).map(|cap| cap[1].to_string())
}

/// Reads the configuration file and returns a mapping of file extensions to printer names.
fn read_config(config_file_path: &str) -> io::Result<Vec<(Regex, String)>> {
    let config_file = std::fs::read_to_string(config_file_path)?;
    let mut mappings = Vec::new();

    for line in config_file.lines() {
        if let Some(delimiter_pos) = line.find('|') {
            let regex_formula = &line[..delimiter_pos];
            let printer_name = &line[delimiter_pos + 1..];

            let re = Regex::new(regex_formula).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            mappings.push((re, printer_name.to_string()));
        }
    }

    Ok(mappings)
}

/// Prints a file on macOS using the `lp` command.
#[cfg(target_os = "macos")]
fn send_print_job(printer_name: &str, document_path: &str) -> io::Result<()> {
    use std::process::Command;

    let status = Command::new("lp")
        .args(&["-d", printer_name, document_path])
        .status()?;

    if status.success() {
        println!("Print job sent successfully to {}.", printer_name);
    } else {
        return Err(io::Error::new(io::ErrorKind::Other, "Failed to execute `lp` command"));
    }

    Ok(())
}

/// Prints a file on Windows using raw printing or an external tool.
#[cfg(target_os = "windows")]
fn send_print_job(printer_name: &str, document_path: &str) -> io::Result<()> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use winapi::um::winspool::{OpenPrinterW, ClosePrinter, StartDocPrinterW, StartPagePrinter, EndPagePrinter, EndDocPrinter, WritePrinter, DOC_INFO_1W};
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::winnt::LPWSTR;
    use winapi::shared::minwindef::{DWORD, BYTE};

    // Convert printer name to wide string
    let printer_name_wide: Vec<u16> = OsStr::new(printer_name).encode_wide().chain(Some(0)).collect();

    // Open the printer
    let mut h_printer = null_mut();
    unsafe {
        if OpenPrinterW(printer_name_wide.as_ptr() as LPWSTR, &mut h_printer, null_mut()) == 0 {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to open printer. Error: {}", GetLastError())));
        }
    }

    // Read the document file as binary data
    let mut file = File::open(document_path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    // Start a print job
    let doc_name: Vec<u16> = OsStr::new("Printing File").encode_wide().chain(Some(0)).collect();
    let raw_datatype: Vec<u16> = OsStr::new("RAW").encode_wide().chain(Some(0)).collect();
    let doc_info = DOC_INFO_1W {
        pDocName: doc_name.as_ptr() as LPWSTR,
        pOutputFile: null_mut(),
        pDatatype: raw_datatype.as_ptr() as LPWSTR,
    };

    unsafe {
        // Cast &doc_info to a mutable pointer
        let job_id = StartDocPrinterW(h_printer, 1, &doc_info as *const _ as *mut BYTE);
        if job_id <= 0 {
            ClosePrinter(h_printer);
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to start print job. Error: {}", GetLastError())));
        }

        // Start a new page
        if StartPagePrinter(h_printer) == 0 {
            EndDocPrinter(h_printer);
            ClosePrinter(h_printer);
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to start page. Error: {}", GetLastError())));
        }

        // Write the print data to the printer
        let mut bytes_written: DWORD = 0;
        // Cast data.as_ptr() to a mutable pointer
        if WritePrinter(h_printer, data.as_ptr() as *mut _, data.len() as DWORD, &mut bytes_written) == 0 {
            EndPagePrinter(h_printer);
            EndDocPrinter(h_printer);
            ClosePrinter(h_printer);
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to write to printer. Error: {}", GetLastError())));
        }

        // End the page
        if EndPagePrinter(h_printer) == 0 {
            EndDocPrinter(h_printer);
            ClosePrinter(h_printer);
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to end page. Error: {}", GetLastError())));
        }

        // End the print job
        if EndDocPrinter(h_printer) == 0 {
            ClosePrinter(h_printer);
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to end print job. Error: {}", GetLastError())));
        }

        // Close the printer
        ClosePrinter(h_printer);
    }

    println!("Print job sent successfully to {}.", printer_name);
    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        return Ok(());
    }

    let file_path = &args[1];

    // Get the path of the executable file
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get executable directory"))?;
    let config_file_path = exe_dir.join("config.txt");

    // Read the configuration file
    let mappings = read_config(config_file_path.to_str().unwrap())?;

    // Get the file extension
    let file_extension = get_file_extension(file_path).unwrap_or_default();

    // Find the appropriate printer for the file extension
    let mut selected_printer: Option<String> = None;
    for (re, printer_name) in mappings {
        if re.is_match(file_path) {
            selected_printer = Some(printer_name.to_string());
            break;
        }
    }

    if let Some(printer_name) = selected_printer {
        println!("Printing to {}", printer_name);
        send_print_job(&printer_name, file_path)?;
    } else {
        eprintln!("No printer found for file extension: {}", file_extension);
    }

    Ok(())
}
