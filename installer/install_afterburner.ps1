# Installs MSI Afterburner via winget if it is not already present.
$paths = @(
    "${env:ProgramFiles(x86)}\MSI Afterburner\MSIAfterburner.exe",
    "$env:ProgramFiles\MSI Afterburner\MSIAfterburner.exe"
)
if ($paths | Where-Object { Test-Path $_ }) {
    Write-Host 'MSI Afterburner already installed.'
    exit 0
}
winget install -e --id Guru3D.Afterburner --silent --accept-source-agreements --accept-package-agreements
exit 0
