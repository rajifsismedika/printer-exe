Set objShell = CreateObject("WScript.Shell")

' Function to check if PDFtoPrinter.exe is running
Function IsProcessRunning(processName)
    Dim strCommand, strOutput
    strCommand = "powershell -Command ""Get-Process " & processName & " -ErrorAction SilentlyContinue"""
    strOutput = objShell.Exec(strCommand).StdOut.ReadAll()
    IsProcessRunning = (InStr(strOutput, processName) > 0)
End Function

' Get the full path to PDFtoPrinter.exe
Dim pdfToPrinterPath
pdfToPrinterPath = "C:\path\to\PDFtoPrinter.exe" ' Update this path

' Check if PDFtoPrinter.exe is already running
If IsProcessRunning("PDFtoPrinter") Then
    WScript.Echo "PDFtoPrinter.exe is already running. Skipping this print job."
Else
    ' Run PDFtoPrinter.exe with the provided arguments
    objShell.Run """" & pdfToPrinterPath & """ """ & WScript.Arguments(0) & """ """ & WScript.Arguments(1) & """", 0, False
End If
