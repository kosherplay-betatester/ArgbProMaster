# Configures OpenRGB to start with Windows the way the daemon needs it:
# elevated (RAM/SMBus + sensor drivers) with the SDK server on — via a
# highest-privileges scheduled task, so there is no UAC prompt at login.
# Then starts it right away, and MSI Afterburner too when present.
$openrgb = @(
    "$env:ProgramFiles\OpenRGB\OpenRGB.exe",
    "${env:ProgramFiles(x86)}\OpenRGB\OpenRGB.exe",
    "$env:LOCALAPPDATA\Programs\OpenRGB\OpenRGB.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1

if ($openrgb) {
    $action = New-ScheduledTaskAction -Execute $openrgb -Argument '--server --startminimized'
    $trigger = New-ScheduledTaskTrigger -AtLogOn
    $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit (New-TimeSpan -Seconds 0)
    Register-ScheduledTask -TaskName 'OpenRGB Autostart' -Action $action -Trigger $trigger -Settings $settings -RunLevel Highest -Force | Out-Null
    # Never spawn a second instance — two OpenRGBs fight over the hardware.
    if (-not (Get-Process -Name OpenRGB -ErrorAction SilentlyContinue)) {
        Start-ScheduledTask -TaskName 'OpenRGB Autostart'
    }
    Write-Host "OpenRGB autostart configured: $openrgb"
} else {
    Write-Host 'OpenRGB not found - the app''s built-in setup assistant will offer to fix this on first run.'
}

$afterburner = @(
    "${env:ProgramFiles(x86)}\MSI Afterburner\MSIAfterburner.exe",
    "$env:ProgramFiles\MSI Afterburner\MSIAfterburner.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if ($afterburner -and -not (Get-Process -Name MSIAfterburner -ErrorAction SilentlyContinue)) {
    Start-Process $afterburner
}
exit 0
