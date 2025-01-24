Set objShell = CreateObject("WScript.Shell")

' Function to check if PDFtoPrinter.exe is running
Function IsProcessRunning(processName)
    Dim strCommand, strOutput
    strCommand = "powershell -Command ""Get-Process " & processName & " -ErrorAction SilentlyContinue"""
    strOutput = objShell.Exec(strCommand).StdOut.ReadAll()
    IsProcessRunning = (InStr(strOutput, processName) > 0)
End Function

' Check if PDFtoPrinter.exe is already running
If IsProcessRunning("PDFtoPrinter") Then
    WScript.Echo "PDFtoPrinter.exe is already running. Skipping this print job."
Else
    ' Run PDFtoPrinter.exe with the provided arguments
    objShell.Run "PDFtoPrinter.exe """ & WScript.Arguments(0) & """ """ & WScript.Arguments(1) & """", 0, False
End If
