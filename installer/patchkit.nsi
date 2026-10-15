; PatchKit — NSIS installer (Modern UI 2).
;
; Build from the repo root:
;   makensis installer\patchkit.nsi
;   makensis /DVERSION=0.2.0 installer\patchkit.nsi
;
; Needs target\release\patchkit.exe (cargo build --release) and the generated
; installer\*.bmp panels + icons\Icon.ico (run tools/gen-assets). Produces
; dist\patchkit-setup.exe.

Unicode true
SetCompressor /SOLID lzma

; All paths below are relative to this script's folder (installer\) — makensis
; resolves them from the script directory, so the build works no matter the
; shell's working directory. ".." is the repo root.

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "WinMessages.nsh"
!include "StrFunc.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif

!define APP_NAME "PatchKit"
!define APP_EXE "patchkit.exe"
!define PUBLISHER "PatchKit"
!define APP_KEY "Software\${APP_NAME}"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"
!define ENV_KEY "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"

Name "${APP_NAME} ${VERSION}"
BrandingText "${APP_NAME}"
OutFile "..\dist\patchkit-setup.exe"
InstallDir "$PROGRAMFILES64\${APP_NAME}"
InstallDirRegKey HKLM "${APP_KEY}" "InstallDir"
RequestExecutionLevel admin
ShowInstDetails show
ShowUninstDetails show

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "FileDescription" "${APP_NAME} installer"
VIAddVersionKey "FileVersion" "${VERSION}.0"
VIAddVersionKey "ProductVersion" "${VERSION}.0"
VIAddVersionKey "LegalCopyright" "${PUBLISHER}"

; Declare the StrFunc string helpers used by the PATH handling below.
; (StrFunc declares a helper and its uninstaller twin with the same name.)
${StrLoc}
${UnStrRep}

; --------------------------------------------------------------- MUI2 pages

!define MUI_ABORTWARNING
!define MUI_ICON "..\icons\Icon.ico"
!define MUI_UNICON "..\icons\Icon.ico"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP "header.bmp"
!define MUI_HEADERIMAGE_RIGHT
!define MUI_WELCOMEFINISHPAGE_BITMAP "welcome.bmp"

!define MUI_WELCOMEPAGE_TITLE "Welcome to the ${APP_NAME} setup"
!define MUI_WELCOMEPAGE_TEXT "This wizard installs ${APP_NAME}, a Windows cleanup, tweaks and downloads TUI.$\r$\n$\r$\nClick Next to continue."

!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Launch ${APP_NAME}"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Var PathValue

; ------------------------------------------------------------------ install

Function .onInit
  SetRegView 64
FunctionEnd

Function un.onInit
  SetRegView 64
FunctionEnd

Section "Install"
  SetShellVarContext all
  SetOutPath "$INSTDIR"
  File "..\target\release\${APP_EXE}"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\${APP_EXE}" 0
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\uninstall.exe" 0
  CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}" "" "$INSTDIR\${APP_EXE}" 0

  WriteRegStr HKLM "${APP_KEY}" "InstallDir" "$INSTDIR"

  WriteRegStr HKLM "${UNINST_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINST_KEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\${APP_EXE}"
  WriteRegStr HKLM "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoRepair" 1

  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKLM "${UNINST_KEY}" "EstimatedSize" "$0"

  Call AddToPath
SectionEnd

; ---------------------------------------------------------------- uninstall

Section "Uninstall"
  SetShellVarContext all
  Call un.RemoveFromPath

  Delete "$DESKTOP\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"

  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  DeleteRegKey HKLM "${UNINST_KEY}"
  DeleteRegKey HKLM "${APP_KEY}"
SectionEnd

; -------------------------------------------------------------- system PATH

; Append $INSTDIR to the machine PATH, skipping it when already present.
Function AddToPath
  ReadRegStr $PathValue HKLM "${ENV_KEY}" "Path"
  ${StrLoc} $0 "$PathValue" "$INSTDIR" ">"
  StrCmp $0 "" 0 AddToPath_done
  StrCmp $PathValue "" 0 AddToPath_append
    StrCpy $PathValue "$INSTDIR"
    Goto AddToPath_write
  AddToPath_append:
    StrCpy $PathValue "$PathValue;$INSTDIR"
  AddToPath_write:
    WriteRegExpandStr HKLM "${ENV_KEY}" "Path" "$PathValue"
    SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=3000
  AddToPath_done:
FunctionEnd

; Remove $INSTDIR from the machine PATH again.
Function un.RemoveFromPath
  ReadRegStr $PathValue HKLM "${ENV_KEY}" "Path"
  ${UnStrRep} $PathValue "$PathValue" "$INSTDIR;" ""
  ${UnStrRep} $PathValue "$PathValue" ";$INSTDIR" ""
  ${UnStrRep} $PathValue "$PathValue" "$INSTDIR" ""
  WriteRegExpandStr HKLM "${ENV_KEY}" "Path" "$PathValue"
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=3000
FunctionEnd
