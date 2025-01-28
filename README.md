# Printer Application

This application is designed for Windows environments to process and send files to a printer, with support for raw printing and PDF files. It utilizes Windows APIs to communicate with printers and requires Rust for development and building.

---

## Prerequisites

### Install Rust
To build and run the application, you need Rust installed on your system. Follow these steps:

1. Download and install Rust:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
2. Restart your terminal or run:
   ```bash
   source $HOME/.cargo/env
   ```
3. Verify the installation:
   ```bash
   rustc --version
   ```

### Install Rust Target for Windows
This application requires the `x86_64-pc-windows-gnu` target for cross-compilation. Install it using:
```bash
rustup target add x86_64-pc-windows-gnu
```

### Install a GCC Compiler (for Windows)
For cross-compiling, you need a GCC compiler such as `mingw-w64`. On macOS, you can install it using Homebrew:
```bash
brew install mingw-w64
```

---

## Configuration

### Configuration File
Create a configuration file named `config.txt` in the same directory as the executable. This file should contain printer mappings in the following format:
```
file_extension_regex|printer_name
```
Example:
```
\.pdf$|PDF_Printer
\.txt$|Text_Printer
```

### PDFtoPrinter
Place the `PDFtoPrinter.exe` tool in the same directory as the executable. This tool is used for printing PDF files.

---

## Building the Application

To build the application for Windows, follow these steps:

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd <repository-folder>
   ```

2. Build the project:
   ```bash
   cargo build --target x86_64-pc-windows-gnu --release
   ```

3. The compiled executable will be located in the `target/x86_64-pc-windows-gnu/release` directory.

---

## Running the Application

1. Navigate to the directory containing the compiled executable.
2. Run the application with the file path as an argument:
   ```bash
   ./Printer.exe <file-path>
   ```
   Example:
   ```bash
   ./Printer.exe C:\Users\User\Documents\example.pdf
   ```
3. The application will determine the appropriate printer based on the configuration file and print the file.

---

## Debugging Tips

- If you encounter issues with the printer handle (`HANDLE` type), ensure you are using the correct Windows target and dependencies.
- For errors related to regular expressions, verify the `config.txt` file for proper syntax and valid regex patterns.

---

## License

This application is provided "as is," without warranty of any kind, express or implied.

---

## Contributing

Contributions are welcome! Feel free to fork the repository and submit a pull request with your changes.

