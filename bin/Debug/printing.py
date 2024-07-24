import os
import sys
import win32print

def send_print_job(printer_name, data):
    try:
        hPrinter = win32print.OpenPrinter(printer_name)
        try:
            hJob = win32print.StartDocPrinter(hPrinter, 1, ("Printing", None, "RAW"))
            try:
                win32print.StartPagePrinter(hPrinter)
                win32print.WritePrinter(hPrinter, data)
                win32print.EndPagePrinter(hPrinter)
            finally:
                win32print.EndDocPrinter(hPrinter)
        finally:
            win32print.ClosePrinter(hPrinter)
        print("Print job sent successfully")
    except Exception as e:
        print("Error:", str(e))

# Example usage
printer_name = win32print.GetDefaultPrinter()

print("Using Printer : ", printer_name)
document_path = "C:\\Users\\rajif\\Documents\\Printer\\bin\\Debug\\test.prn"

# Read the document file as binary data
with open(document_path, "rb") as file:
    data = file.read()

# print("Data is : ", data)

# Send the print job
send_print_job(printer_name, data)
