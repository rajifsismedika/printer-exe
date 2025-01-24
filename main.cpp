#include <windows.h>
#include <winspool.h>
#include <iostream>
#include <fstream>
#include <regex>
#include <tchar.h>
#include <vector>
#include <shellapi.h> // For CommandLineToArgvW and ShellExecute

// Function to send a raw print job
void SendPrintRawJob(LPTSTR printerName, const std::string& documentPath)
{
    HANDLE hPrinter = NULL;

    // Read the document file as binary data
    std::ifstream file(documentPath, std::ios::binary);
    if (!file)
    {
        MessageBox(NULL, _T("Failed to open the document file."), _T("Error"), MB_ICONERROR);
        return;
    }

    std::vector<BYTE> data(std::istreambuf_iterator<char>(file), {});

    // Open the printer
    if (!OpenPrinter(printerName, &hPrinter, NULL))
    {
        MessageBox(NULL, _T("Failed to open the printer."), _T("Error"), MB_ICONERROR);
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
        MessageBox(NULL, _T("Failed to start the print job."), _T("Error"), MB_ICONERROR);
        ClosePrinter(hPrinter);
        return;
    }

    // Start a new page
    if (!StartPagePrinter(hPrinter))
    {
        MessageBox(NULL, _T("Failed to start a new page."), _T("Error"), MB_ICONERROR);
        EndDocPrinter(hPrinter);
        ClosePrinter(hPrinter);
        return;
    }

    // Write the print data to the printer
    DWORD bytesWritten = 0;
    if (!WritePrinter(hPrinter, data.data(), static_cast<DWORD>(data.size()), &bytesWritten))
    {
        MessageBox(NULL, _T("Failed to write to the printer."), _T("Error"), MB_ICONERROR);
        EndPagePrinter(hPrinter);
        EndDocPrinter(hPrinter);
        ClosePrinter(hPrinter);
        return;
    }

    // End the page
    if (!EndPagePrinter(hPrinter))
    {
        MessageBox(NULL, _T("Failed to end the page."), _T("Error"), MB_ICONERROR);
        EndDocPrinter(hPrinter);
        ClosePrinter(hPrinter);
        return;
    }

    // End the print job
    if (!EndDocPrinter(hPrinter))
    {
        MessageBox(NULL, _T("Failed to end the print job."), _T("Error"), MB_ICONERROR);
        ClosePrinter(hPrinter);
        return;
    }

    // Close the printer
    ClosePrinter(hPrinter);

    MessageBox(NULL, _T("Print job sent successfully."), _T("Success"), MB_ICONINFORMATION);
}

// Function to get the file extension
std::string GetFileExtension(const std::string& filePath)
{
    std::regex pattern("\\.([a-zA-Z0-9]+)$");
    std::smatch match;

    if (std::regex_search(filePath, match, pattern))
    {
        return match[1].str();
    }

    return "";
}

// Function to send a PDF print job
void SendPrintPdfJob(LPTSTR printerName, const std::string& documentPath)
{
    std::string command = "PDFtoPrinter.exe \"" + documentPath + "\" \"" + printerName + "\"";

    STARTUPINFO si;
    PROCESS_INFORMATION pi;

    ZeroMemory(&si, sizeof(si));
    si.cb = sizeof(si);
    si.dwFlags = STARTF_USESHOWWINDOW;
    si.wShowWindow = SW_HIDE;

    ZeroMemory(&pi, sizeof(pi));

    // Create the process with CREATE_NO_WINDOW flag to hide the terminal window
    if (!CreateProcess(
            NULL,
            const_cast<char*>(command.c_str()),
            NULL,
            NULL,
            FALSE,
            CREATE_NO_WINDOW,
            NULL,
            NULL,
            &si,
            &pi))
    {
        MessageBox(NULL, _T("Failed to execute PDFtoPrinter.exe."), _T("Error"), MB_ICONERROR);
        return;
    }

    // Wait for the process to finish
    WaitForSingleObject(pi.hProcess, INFINITE);

    // Close process and thread handles
    CloseHandle(pi.hProcess);
    CloseHandle(pi.hThread);

    MessageBox(NULL, _T("Print job sent successfully."), _T("Success"), MB_ICONINFORMATION);
}

// Function to display printer names
int showPrinterNames()
{
    DWORD numPrinters;
    DWORD bufferSize = 0;

    // Call EnumPrinters with level 4 to list all printers
    EnumPrinters(PRINTER_ENUM_LOCAL, nullptr, 4, nullptr, 0, &bufferSize, &numPrinters);

    // Allocate memory to hold the printer information
    PRINTER_INFO_4* printerInfo = reinterpret_cast<PRINTER_INFO_4*>(new BYTE[bufferSize]);

    // Call EnumPrinters again to get the printer information
    EnumPrinters(PRINTER_ENUM_LOCAL, nullptr, 4, reinterpret_cast<LPBYTE>(printerInfo), bufferSize, &bufferSize, &numPrinters);

    if (numPrinters > 0)
    {
        std::string printerList = "List of printers:\n";
        for (DWORD i = 0; i < numPrinters; ++i)
        {
            printerList += std::string(printerInfo[i].pPrinterName) + "\n";
        }
        MessageBox(NULL, std::wstring(printerList.begin(), printerList.end()).c_str(), _T("Printers"), MB_ICONINFORMATION);
    }
    else
    {
        MessageBox(NULL, _T("No printers found."), _T("Printers"), MB_ICONINFORMATION);
    }

    // Clean up allocated memory
    delete[] printerInfo;

    return 0;
}

// Entry point for Windows application
int APIENTRY WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow)
{
    // Parse command-line arguments
    int argc;
    LPWSTR* argv = CommandLineToArgvW(GetCommandLineW(), &argc);
    if (!argv)
    {
        MessageBox(NULL, _T("Failed to parse command-line arguments."), _T("Error"), MB_ICONERROR);
        return 1;
    }

    std::string filename = (argc > 1) ? std::string(argv[1]) : "";

    // Call showPrinterNames (optional, if you still want to list printers)
    // showPrinterNames();

    // Get the path of the executable file
    TCHAR exePath[MAX_PATH];
    GetModuleFileName(NULL, exePath, MAX_PATH);

    std::string exeDir = std::string(exePath);
    std::size_t lastSlash = exeDir.find_last_of("\\/");
    std::string exeDirectory = exeDir.substr(0, lastSlash);

    std::string configFilePath = exeDirectory + "\\config.txt";
    std::ifstream inputFile(configFilePath);

    if (inputFile.is_open())
    {
        std::string line;
        std::string selectedPrinterName = "No Printer Selected";
        std::string fileExtension = "";

        while (std::getline(inputFile, line))
        {
            size_t delimiterPos = line.find('|');
            if (delimiterPos != std::string::npos)
            {
                std::string regexFormula = line.substr(0, delimiterPos);
                std::string printerName = line.substr(delimiterPos + 1);

                std::regex pattern(regexFormula, std::regex_constants::icase);
                if (std::regex_search(filename, pattern))
                {
                    selectedPrinterName = printerName;
                    fileExtension = GetFileExtension(filename);
                    break;
                }
            }
        }

        inputFile.close();

        if (fileExtension == "pdf")
        {
            SendPrintPdfJob(const_cast<LPTSTR>(selectedPrinterName.c_str()), filename);
        }
        else
        {
            SendPrintRawJob(const_cast<LPTSTR>(selectedPrinterName.c_str()), filename);
        }
    }
    else
    {
        MessageBox(NULL, _T("Unable to open the config file."), _T("Error"), MB_ICONERROR);
    }

    LocalFree(argv);
    return 0;
}
