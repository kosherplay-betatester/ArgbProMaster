# Installs OpenRGB via winget if it is not already present.
$paths = @(
    "$env:ProgramFiles\OpenRGB\OpenRGB.exe",
    "${env:ProgramFiles(x86)}\OpenRGB\OpenRGB.exe",
    "$env:LOCALAPPDATA\Programs\OpenRGB\OpenRGB.exe"
)
if ($paths | Where-Object { Test-Path $_ }) {
    Write-Host 'OpenRGB already installed.'
    exit 0
}
winget install -e --id OpenRGB.OpenRGB --silent --accept-source-agreements --accept-package-agreements
exit 0
