!include "MUI2.nsh"

Name "Kobo Highlights Exporter"
OutFile "kobo-highlights-exporter-installer.exe"
InstallDir "$PROGRAMFILES\Kobo Highlights Exporter"
InstallDirRegKey HKCU "Software\Kobo Highlights Exporter" "InstallDir"
RequestExecutionLevel user

; Installer UI settings
!define MUI_ICON "kobo-highlights-icon.ico"
!define MUI_UNICON "kobo-highlights-icon.ico"
!define MUI_ABORTWARNING

; Installer pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

; Uninstaller pages
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "Install"
    SetOutPath "$INSTDIR"

    ; Install files
    File "kobo-highlights-exporter.exe"
    File "kobo-highlights-icon.ico"

    ; Write install directory to registry
    WriteRegStr HKCU "Software\Kobo Highlights Exporter" "InstallDir" "$INSTDIR"

    ; Add right-click context menu for .sqlite files
    WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.sqlite\shell\RunKoboHighlights" "" "Run Kobo Highlights Exporter"
    WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.sqlite\shell\RunKoboHighlights" "Icon" "$INSTDIR\kobo-highlights-icon.ico"
    WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.sqlite\shell\RunKoboHighlights\command" "" '"$INSTDIR\kobo-highlights-exporter.exe" "%1"'

    ; Create uninstaller
    WriteUninstaller "$INSTDIR\uninstall.exe"

    ; Add entry to "Add or Remove Programs"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\KoboHighlightsExporter" "DisplayName" "Kobo Highlights Exporter"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\KoboHighlightsExporter" "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\KoboHighlightsExporter" "DisplayIcon" "$INSTDIR\kobo-highlights-icon.ico"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\KoboHighlightsExporter" "Publisher" "mrRo8o7"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\KoboHighlightsExporter" "InstallLocation" "$INSTDIR"
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\KoboHighlightsExporter" "NoModify" 1
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\KoboHighlightsExporter" "NoRepair" 1
SectionEnd

Section "Uninstall"
    ; Remove right-click context menu
    DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.sqlite\shell\RunKoboHighlights"

    ; Remove "Add or Remove Programs" entry
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\KoboHighlightsExporter"

    ; Remove install directory registry key
    DeleteRegKey HKCU "Software\Kobo Highlights Exporter"

    ; Remove files and directory
    Delete "$INSTDIR\kobo-highlights-exporter.exe"
    Delete "$INSTDIR\kobo-highlights-icon.ico"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR"
SectionEnd
