#include <windows.h>
#include <winspool.h>
#include <iostream>
#include <fstream>
#include <regex>
#include <tchar.h>
#include <vector>


void SendPrintRawJob(LPTSTR printerName, std::string& documentPath)
{
    // Winspool -static -static-libgcc -static-libstdc++

    HANDLE hPrinter = NULL;

    // Read the document file as binary data
    std::ifstream file(documentPath, std::ios::binary);
    if (!file)
    {
        std::cout << "Failed to open the document file." << std::endl;
        return;
    }

    std::vector<BYTE> data(std::istreambuf_iterator<char>(file), {});

    // Open the printer
    if (!OpenPrinter(printerName, &hPrinter, NULL))
    {
        std::cout << "Failed to open the printer. Error: " << GetLastError() << std::endl;
        return;
    }

    // Start a print job
    DOC_INFO_1 docInfo;
    docInfo.pDocName = _T("Printing File");
    docInfo.pOutputFile = NULL;
    docInfo.pDatatype = _T("RAW");

    DWORD jobId = StartDocPrinter(hPrinter, 1, reinterpret_cast<LPBYTE>(&docInfo));
    if (jobId <= 0)
    {
        std::cout << "Failed to start the print job. Error: " << GetLastError() << std::endl;
        ClosePrinter(hPrinter);
        return;
    }

    // Start a new page
    if (!StartPagePrinter(hPrinter))
    {
        std::cout << "Failed to start a new page. Error: " << GetLastError() << std::endl;
        EndDocPrinter(hPrinter);
        ClosePrinter(hPrinter);
        return;
    }

    // Write the print data to the printer
    DWORD bytesWritten = 0;
    if (!WritePrinter(hPrinter, data.data(), static_cast<DWORD>(data.size()), &bytesWritten))
    {
        std::cout << "Failed to write to the printer. Error: " << GetLastError() << std::endl;
        EndPagePrinter(hPrinter);
        EndDocPrinter(hPrinter);
        ClosePrinter(hPrinter);
        return;
    }

    // End the page
    if (!EndPagePrinter(hPrinter))
    {
        std::cout << "Failed to end the page. Error: " << GetLastError() << std::endl;
        EndDocPrinter(hPrinter);
        ClosePrinter(hPrinter);
        return;
    }

    // End the print job
    if (!EndDocPrinter(hPrinter))
    {
        std::cout << "Failed to end the print job. Error: " << GetLastError() << std::endl;
        ClosePrinter(hPrinter);
        return;
    }

    // Close the printer
    ClosePrinter(hPrinter);

    std::cout << "Print job sent successfully." << std::endl;
}

std::string GetFileExtension(const std::string& filePath)
{
    // Regular expression pattern to match the file extension
    std::regex pattern("\\.([a-zA-Z0-9]+)$");

    // Match object to store the results
    std::smatch match;

    // Perform the regex search
    if (std::regex_search(filePath, match, pattern))
    {
        // Return the matched file extension
        return match[1].str();
    }

    // Return an empty string if no match found
    return "";
}

void SendPrintPdfJob(LPTSTR printerName, const std::string& documentPath)
{
    std::string command = "PDFtoPrinter.exe \"" + documentPath + "\" \"" + printerName + "\"";

    STARTUPINFO si;
    PROCESS_INFORMATION pi;

    ZeroMemory(&si, sizeof(si));
    si.cb = sizeof(si);
    ZeroMemory(&pi, sizeof(pi));

    // Create the process with CREATE_NO_WINDOW flag to hide the terminal window
    if (!CreateProcess(NULL, const_cast<char*>(command.c_str()), NULL, NULL, FALSE, CREATE_NO_WINDOW, NULL, NULL, &si, &pi))
    {
        std::cout << "Failed to execute PDFtoPrinter.exe. Error: " << GetLastError() << std::endl;
        return;
    }

    // Wait for the process to finish
    WaitForSingleObject(pi.hProcess, INFINITE);

    // Close process and thread handles
    CloseHandle(pi.hProcess);
    CloseHandle(pi.hThread);

    std::cout << "Print job sent successfully." << std::endl;
}

int showPrinterNames() {
    DWORD numPrinters;
    DWORD bufferSize = 0;
    DWORD i;

    // Call EnumPrinters with level 4 to list all printers
    EnumPrinters(PRINTER_ENUM_LOCAL, nullptr, 4, nullptr, 0, &bufferSize, &numPrinters);

    // Allocate memory to hold the printer information
    PRINTER_INFO_4* printerInfo = reinterpret_cast<PRINTER_INFO_4*>(new BYTE[bufferSize]);

    // Call EnumPrinters again to get the printer information
    EnumPrinters(PRINTER_ENUM_LOCAL, nullptr, 4, reinterpret_cast<LPBYTE>(printerInfo), bufferSize, &bufferSize, &numPrinters);

    if (numPrinters > 0) {
        std::cout << "List of printers:\n";
        for (i = 0; i < numPrinters; ++i) {
            std::wcout << printerInfo[i].pPrinterName << "\n";
        }
    } else {
        std::cout << "No printers found.\n";
    }

    // Clean up allocated memory
    delete[] printerInfo;

    return 0;
}

int main(int argc, char* argv[]) {
    std::string filename = (argc > 1) ? argv[1] : "";

    showPrinterNames();

    std::cout << "Params: " << filename << std::endl;

    // Get the path of the executable file
    TCHAR exePath[MAX_PATH];
    GetModuleFileName(NULL, exePath, MAX_PATH);

    std::string exeDir = std::string(exePath);
    std::size_t lastSlash = exeDir.find_last_of("\\/");
    std::string exeDirectory = exeDir.substr(0, lastSlash);

    std::string configFilePath = exeDirectory + "\\config.txt";
    std::ifstream inputFile(configFilePath);

    if (inputFile.is_open()) {
        std::string line;
        std::string fileContent;
        std::string selectedPrinterName = "No Printer Selected"; // Example printer name
        std::string fileExtension = "";


        std::cout << "Processing File: " << filename << std::endl;

        while (std::getline(inputFile, line)) {
            fileContent += line + "\n";

            size_t delimiterPos = line.find('|');
            if (delimiterPos != std::string::npos) {
                std::string regexFormula = line.substr(0, delimiterPos);
                std::string printerName = line.substr(delimiterPos + 1);
                std::cout << "Testing against: " << regexFormula << std::endl;

                std::regex pattern(regexFormula, std::regex_constants::icase);
                if (std::regex_search(filename, pattern)) {
                    selectedPrinterName = printerName;
                    fileExtension = GetFileExtension(filename);
                    break;
                }
            }
        }

        inputFile.close();

        std::cout << "Selected Printer Name: " << selectedPrinterName << std::endl;
        std::cout << "Extension: " << fileExtension << std::endl;

        if (fileExtension == "pdf") {
            std::cout << "Printing PDF"  << std::endl;
            SendPrintPdfJob(const_cast<LPTSTR>(selectedPrinterName.c_str()), filename);
        } else {
            std::cout << "Printing RAW"  << std::endl;
            SendPrintRawJob(const_cast<LPTSTR>(selectedPrinterName.c_str()), filename);
        }
    } else {
        std::cout << "Unable to open the file." << std::endl;
    }

    return 0;
}
