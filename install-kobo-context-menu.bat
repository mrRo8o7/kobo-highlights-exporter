@echo off
echo Installing Kobo Highlights Exporter context menu...

:: Create registry file with current user path
echo Windows Registry Editor Version 5.00 > "%TEMP%\kobo-highlights-menu.reg"
echo. >> "%TEMP%\kobo-highlights-menu.reg"
echo ; Add context menu entry for .sqlite files with custom icon >> "%TEMP%\kobo-highlights-menu.reg"
echo [HKEY_CURRENT_USER\Software\Classes\SystemFileAssociations\.sqlite\shell\RunKoboHighlights] >> "%TEMP%\kobo-highlights-menu.reg"
echo @="Run Kobo Highlights Exporter" >> "%TEMP%\kobo-highlights-menu.reg"
echo "Icon"="%USERPROFILE%\\kobo-highlights-exporter\\kobo-highlights-icon.ico" >> "%TEMP%\kobo-highlights-menu.reg"
echo. >> "%TEMP%\kobo-highlights-menu.reg"
echo [HKEY_CURRENT_USER\Software\Classes\SystemFileAssociations\.sqlite\shell\RunKoboHighlights\command] >> "%TEMP%\kobo-highlights-menu.reg"
echo @="\"%USERPROFILE%\\kobo-highlights-exporter\\kobo-highlights-exporter.exe\" \"%%1\"" >> "%TEMP%\kobo-highlights-menu.reg"


:: Import the registry file
reg import "%TEMP%\kobo-highlights-menu.reg"

if %errorlevel% == 0 (
    echo.
    echo ✓ Context menu installed successfully!
    echo ✓ Make sure to copy kobo-highlights-icon.ico to: %USERPROFILE%\kobo-highlights-exporter\
    echo.
    echo Right-click any .sqlite file to see "Run Kobo Highlights Exporter"
) else (
    echo.
    echo ✗ Installation failed. Please run as administrator.
)

:: Clean up temporary file
del "%TEMP%\kobo-highlights-menu.reg"

echo.
pause
