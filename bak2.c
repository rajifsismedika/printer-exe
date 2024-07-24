#include <windows.h>
#include <winspool.h>
#include <tchar.h>
#include <iostream>
#include <fstream>

void PrintJob(LPTSTR printerName, LPTSTR documentPath)
{
    HANDLE printerHandle = NULL;

    // Open the printer
    if (!OpenPrinter(printerName, &printerHandle, NULL))
    {
        std::cout << "Failed to open the printer. Error: " << GetLastError() << std::endl;
        return;
    }

    // Start a print job
    DOC_INFO_1 docInfo;
    docInfo.pDocName = documentPath;
    docInfo.pOutputFile = NULL;
    docInfo.pDatatype = _T("IMAGE/JPEG"); // Set the datatype to "IMAGE/JPEG"

    DWORD printJobId = StartDocPrinter(printerHandle, 1, reinterpret_cast<LPBYTE>(&docInfo));
    if (printJobId == 0)
    {
        std::cout << "Failed to start the print job. Error: " << GetLastError() << std::endl;
        ClosePrinter(printerHandle);
        return;
    }

    // Open the document file
    HANDLE fileHandle = CreateFile(documentPath, GENERIC_READ, 0, NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    if (fileHandle == INVALID_HANDLE_VALUE)
    {
        std::cout << "Failed to open the document file. Error: " << GetLastError() << std::endl;
        EndDocPrinter(printerHandle);
        ClosePrinter(printerHandle);
        return;
    }

    // Read the document file and send the print data to the printer
    DWORD bytesRead = 0;
    BYTE buffer[4096];
    while (ReadFile(fileHandle, buffer, sizeof(buffer), &bytesRead, NULL) && bytesRead > 0)
    {
        DWORD bytesWritten = 0;
        if (!WritePrinter(printerHandle, buffer, bytesRead, &bytesWritten))
        {
            std::cout << "Failed to write to the printer. Error: " << GetLastError() << std::endl;
            CloseHandle(fileHandle);
            EndDocPrinter(printerHandle);
            ClosePrinter(printerHandle);
            return;
        }
    }

    // Close the document file
    CloseHandle(fileHandle);

    // End the print job
    if (!EndDocPrinter(printerHandle))
    {
        std::cout << "Failed to end the print job. Error: " << GetLastError() << std::endl;
        ClosePrinter(printerHandle);
        return;
    }

    // Close the printer
    ClosePrinter(printerHandle);

    std::cout << "Print job sent successfully." << std::endl;
}

int printerList()
{
    // Get the list of printers
    DWORD numPrinters = 0;
    DWORD bufferSize = 0;

    EnumPrinters(PRINTER_ENUM_LOCAL, NULL, 2, NULL, 0, &bufferSize, &numPrinters);

    if (bufferSize == 0)
    {
        std::cout << "No printers found." << std::endl;
        return 0;
    }

    PRINTER_INFO_2* printerInfo = new PRINTER_INFO_2[bufferSize];
    if (!EnumPrinters(PRINTER_ENUM_LOCAL, NULL, 2, reinterpret_cast<LPBYTE>(printerInfo), bufferSize, &bufferSize, &numPrinters))
    {
        std::cout << "Failed to retrieve printer list." << std::endl;
        delete[] printerInfo;
        return 1;
    }

    // Display the list of printers
    std::cout << "Printers:" << std::endl;
    for (DWORD i = 0; i < numPrinters; ++i)
    {
        std::cout << printerInfo[i].pPrinterName << std::endl;
    }

    delete[] printerInfo;

    return 0;
}

int main(int argc, _TCHAR* argv[])
{
    // std::ofstream outputFile("output.txt");
    // std::streambuf* coutBuffer = std::cout.rdbuf();
    // std::cout.rdbuf(outputFile.rdbuf());

    if (argc < 3)
    {
        printerList();
        std::cout << "Usage: " << argv[0] << " <printerName> <documentPath>" << std::endl;
        system("pause");
        return 1;
    }

    LPTSTR printerName = argv[1];
    LPTSTR documentPath = argv[2];

    std::cout << "printerName: " << argv[1] << ", documentPath: " << argv[2] << std::endl;

    PrintJob(printerName, documentPath);

    //std::cout.rdbuf(coutBuffer);
    //outputFile.close();

    return 0;
}
