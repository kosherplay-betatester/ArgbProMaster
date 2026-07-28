; ArgbProMaster one-click installer (Inno Setup 6).
; Build:  ISCC.exe ArgbProMaster.iss     (after `cargo build --release`)
; Output: Output\ArgbProMaster-Setup-v<version>.exe
;
; What it does, A to Z:
;   - installs the app to Program Files with Start Menu / Desktop shortcuts
;   - installs OpenRGB and MSI Afterburner via winget (skipped if present)
;   - configures OpenRGB to start with Windows (admin + SDK server, via a
;     scheduled task so there is no UAC prompt at login) and starts it now
;   - sets the lighting daemon to start with Windows and starts it now
;   - launches the configurator at the end

#define MyAppVersion "1.3"

[Setup]
AppId={{8F7C1D2E-4A5B-4C6D-9E0F-1A2B3C4D5E6F}
AppName=ArgbProMaster
AppVersion={#MyAppVersion}
AppPublisher=ArgbProMaster contributors
AppPublisherURL=https://github.com/kosherplay-betatester/ArgbProMaster
AppSupportURL=https://github.com/kosherplay-betatester/ArgbProMaster
DefaultDirName={autopf}\ArgbProMaster
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
OutputBaseFilename=ArgbProMaster-Setup-v{#MyAppVersion}
Compression=lzma2
SolidCompression=yes
PrivilegesRequired=admin
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
UninstallDisplayIcon={app}\configurator_gui.exe

[Files]
Source: "..\target\release\configurator_gui.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\thermal_daemon.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"
Source: "..\README.md"; DestDir: "{app}"
Source: "..\CHANGELOG.md"; DestDir: "{app}"
Source: "install_openrgb.ps1"; DestDir: "{app}\setup"
Source: "install_afterburner.ps1"; DestDir: "{app}\setup"
Source: "setup_openrgb_task.ps1"; DestDir: "{app}\setup"

[Tasks]
Name: "installopenrgb"; Description: "Install OpenRGB (required — the engine that talks to your LEDs)"
Name: "installafterburner"; Description: "Install MSI Afterburner (recommended — provides CPU/GPU temperatures)"
Name: "openrgbtask"; Description: "Start OpenRGB with Windows (admin + SDK server, no login prompts)"
Name: "daemonstart"; Description: "Start the lighting daemon with Windows"
Name: "desktopicon"; Description: "Create a Desktop shortcut"

[Icons]
Name: "{autoprograms}\ArgbProMaster"; Filename: "{app}\configurator_gui.exe"
Name: "{autodesktop}\ArgbProMaster"; Filename: "{app}\configurator_gui.exe"; Tasks: desktopicon
Name: "{userstartup}\ArgbProMaster Daemon"; Filename: "{app}\thermal_daemon.exe"; Tasks: daemonstart

[Run]
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File ""{app}\setup\install_openrgb.ps1"""; StatusMsg: "Installing OpenRGB (can take a minute)…"; Tasks: installopenrgb; Flags: runhidden waituntilterminated
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File ""{app}\setup\install_afterburner.ps1"""; StatusMsg: "Installing MSI Afterburner (can take a minute)…"; Tasks: installafterburner; Flags: runhidden waituntilterminated
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File ""{app}\setup\setup_openrgb_task.ps1"""; StatusMsg: "Configuring OpenRGB to start with Windows…"; Tasks: openrgbtask; Flags: runhidden waituntilterminated
Filename: "{app}\thermal_daemon.exe"; StatusMsg: "Starting the lighting daemon…"; Tasks: daemonstart; Flags: nowait
Filename: "{app}\configurator_gui.exe"; Description: "Launch ArgbProMaster now"; Flags: postinstall nowait skipifsilent

[UninstallRun]
Filename: "taskkill.exe"; Parameters: "/IM thermal_daemon.exe /F"; Flags: runhidden; RunOnceId: "KillDaemon"
Filename: "schtasks.exe"; Parameters: "/Delete /TN ""OpenRGB Autostart"" /F"; Flags: runhidden; RunOnceId: "DropTask"
