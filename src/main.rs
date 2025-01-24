use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, Read};
use std::ptr::null_mut;
use std::os::windows::ffi::OsStrExt;
use std::process::Command;
use std::sync::Mutex;
use lazy_static::lazy_static;
use regex::Regex;
use winapi::um::winspool::{OpenPrinterW, ClosePrinter, StartDocPrinterW, StartPagePrinter, EndPagePrinter, EndDocPrinter, WritePrinter, DOC_INFO_1W};
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::winnt::LPWSTR;
use winapi::shared::minwindef::{DWORD, BYTE};

// Global queue for print jobs
lazy_static! {
    static ref PRINT_QUEUE: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
}

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

/// Sends a raw print job to the specified printer.
fn send_print_raw_job(printer_name: &str, document_path: &str) -> io::Result<()> {
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

    Ok(())
}

/// Prints a file using the appropriate method based on its extension.
fn send_print_job(printer_name: &str, document_path: &str) -> io::Result<()> {
    let file_extension = get_file_extension(document_path).unwrap_or_default();

    if file_extension == "pdf" {
        // Use PDFtoPrinter.exe for PDF files
        // Remove leading and trailing backslashes from the printer name
        let trimmed_printer_name = printer_name.trim_matches('\\');

        // Debugging: Print the trimmed printer name
        println!("Trimmed printer name: {}", trimmed_printer_name);

        // Construct the command with the properly formatted printer name
        let command = "PDFtoPrinter.exe"; // Directly use the executable
        let args = [document_path, trimmed_printer_name];

        // Debugging: Print the command and arguments
        println!("Executing command: {} \"{}\" \"{}\"", command, args[0], args[1]);

        // Execute the command directly
        let output = Command::new(command)
            .args(&args)
            .output()?;

        // Debugging: Print the command output
        println!("Command output: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("Command error: {}", String::from_utf8_lossy(&output.stderr));

        if output.status.success() {
            println!("Print job sent successfully to {}.", trimmed_printer_name);
        } else {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to execute PDFtoPrinter.exe"));
        }
    } else {
        // Use raw printing for non-PDF files
        send_print_raw_job(printer_name, document_path)?;
    }

    Ok(())
}

/// Adds a print job to the queue.
fn add_print_job(printer_name: String, document_path: String) {
    PRINT_QUEUE.lock().unwrap().push((printer_name, document_path));
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
    let mut selected_printer = None;
    for (re, printer_name) in mappings {
        if re.is_match(file_path) {
            selected_printer = Some(printer_name);
            break;
        }
    }

    if let Some(printer_name) = selected_printer {
        // Add the print job to the queue
        add_print_job(printer_name, file_path.to_string());

        // Process the print queue
        process_print_queue();
    } else {
        eprintln!("No printer found for file extension: {}", file_extension);
    }

    Ok(())
}
