; ArgbProMaster one-click installer (Inno Setup 6).
; Build:  ISCC.exe ArgbProMaster.iss     (after `cargo build --release`)
; Output: Output\ArgbProMaster-Setup-v<version>.exe
;
; What it does, A to Z:
;   - installs the app to Program Files with Start Menu / Desktop shortcuts
;   - upgrades cleanly: closes a running daemon/configurator by itself first,
;     and restarts the daemon afterwards
;   - installs OpenRGB and MSI Afterburner via winget — each checkbox tells
;     the user when the software is already installed
;   - configures OpenRGB to start with Windows (admin + SDK server, via a
;     scheduled task so there is no UAC prompt at login) and starts it now;
;     the checkbox tells the user when this is already set up
;   - sets the lighting daemon to start with Windows (same notification)
;   - launches the configurator at the end

#define MyAppVersion "1.4.2"

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
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\configurator_gui.exe
; Upgrades: we close our own processes in PrepareToInstall (below), so the
; stock "applications are using files" page never needs to appear.
CloseApplications=no

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
Name: "installopenrgb"; Description: "{code:OpenRgbTaskDesc}"
Name: "installafterburner"; Description: "{code:AfterburnerTaskDesc}"
Name: "openrgbtask"; Description: "{code:OpenRgbAutostartDesc}"
Name: "daemonstart"; Description: "{code:DaemonAutostartDesc}"
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

[Code]
var
  HaveOpenRgb: Boolean;
  HaveAfterburner: Boolean;
  HaveOpenRgbTask: Boolean;
  HaveDaemonShortcut: Boolean;
  DaemonWasRunning: Boolean;

{ Detect everything once, so the task checkboxes can tell the user what is
  already in place (typical when upgrading from an older version). }
function InitializeSetup(): Boolean;
var
  RC: Integer;
begin
  HaveOpenRgb :=
    FileExists(ExpandConstant('{commonpf64}\OpenRGB\OpenRGB.exe')) or
    FileExists(ExpandConstant('{commonpf32}\OpenRGB\OpenRGB.exe')) or
    FileExists(ExpandConstant('{localappdata}\Programs\OpenRGB\OpenRGB.exe'));
  HaveAfterburner :=
    FileExists(ExpandConstant('{commonpf32}\MSI Afterburner\MSIAfterburner.exe')) or
    FileExists(ExpandConstant('{commonpf64}\MSI Afterburner\MSIAfterburner.exe'));
  { The start-with-Windows pieces a previous install may have set up. }
  HaveOpenRgbTask :=
    Exec(ExpandConstant('{sys}\schtasks.exe'), '/Query /TN "OpenRGB Autostart"', '',
      SW_HIDE, ewWaitUntilTerminated, RC) and (RC = 0);
  HaveDaemonShortcut :=
    FileExists(ExpandConstant('{userstartup}\ArgbProMaster Daemon.lnk'));
  DaemonWasRunning := False;
  Result := True;
end;

function OpenRgbTaskDesc(Param: String): String;
begin
  if HaveOpenRgb then
    Result := 'OpenRGB: already installed - nothing to download (tick only to re-check it)'
  else
    Result := 'Install OpenRGB (required - the engine that talks to your LEDs)';
end;

function AfterburnerTaskDesc(Param: String): String;
begin
  if HaveAfterburner then
    Result := 'MSI Afterburner: already installed - nothing to download (tick only to re-check it)'
  else
    Result := 'Install MSI Afterburner (recommended - provides CPU/GPU temperatures)';
end;

function OpenRgbAutostartDesc(Param: String): String;
begin
  if HaveOpenRgbTask then
    Result := 'OpenRGB start-with-Windows: already set up from a previous install (tick to re-apply)'
  else
    Result := 'Start OpenRGB with Windows (admin + SDK server, no login prompts)';
end;

function DaemonAutostartDesc(Param: String): String;
begin
  if HaveDaemonShortcut then
    Result := 'Lighting daemon start-with-Windows: already set up from a previous install (tick to re-apply)'
  else
    Result := 'Start the lighting daemon with Windows';
end;

{ Anything reported as "already ..." starts unticked, so an upgrade skips
  straight past what is in place. Ticking it anyway is always harmless -
  every step is idempotent. }
procedure CurPageChanged(CurPageID: Integer);
var
  i: Integer;
begin
  if CurPageID = wpSelectTasks then
    for i := 0 to WizardForm.TasksList.Items.Count - 1 do
      if Pos('already', WizardForm.TasksList.ItemCaption[i]) > 0 then
        WizardForm.TasksList.Checked[i] := False;
end;

{ Upgrades used to fail on locked files when the daemon (or a tray
  configurator) was still running - close both ourselves, right before the
  files are copied. }
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  RC: Integer;
begin
  if Exec(ExpandConstant('{sys}\taskkill.exe'), '/IM thermal_daemon.exe /F', '',
       SW_HIDE, ewWaitUntilTerminated, RC) then
    DaemonWasRunning := (RC = 0);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/IM configurator_gui.exe /F', '',
    SW_HIDE, ewWaitUntilTerminated, RC);
  { Give Windows a moment to release the file handles. }
  Sleep(800);
  Result := '';
end;

{ If we had to stop a running daemon for the upgrade, make sure it comes
  back even when the start-with-Windows task is left unticked. Double
  starts are safe - the daemon refuses to run twice. }
procedure CurStepChanged(CurStep: TSetupStep);
var
  RC: Integer;
begin
  if (CurStep = ssPostInstall) and DaemonWasRunning
     and not WizardIsTaskSelected('daemonstart') then
    Exec(ExpandConstant('{app}\thermal_daemon.exe'), '', '', SW_HIDE, ewNoWait, RC);
end;
