param(
  [string]$MsiPath = "",
  [switch]$LaunchOnly
)

$ErrorActionPreference = "Stop"

function Find-InstalledWhisperingExe {
  $candidateRoots = @(
    $env:LOCALAPPDATA,
    $env:ProgramFiles,
    ${env:ProgramFiles(x86)}
  ) | Where-Object { $_ }

  $candidates = @()

  foreach ($root in $candidateRoots) {
    if ($root -eq $env:LOCALAPPDATA) {
      $candidates += Join-Path $root "Programs\Whispering\Whispering.exe"
      continue
    }

    $candidates += Join-Path $root "Whispering\Whispering.exe"
  }

  $candidates = $candidates | Where-Object { Test-Path $_ }

  if ($candidates.Count -gt 0) {
    return $candidates[0]
  }

  return $null
}

if ($env:OS -ne "Windows_NT") {
  throw "install-windows-app.ps1 is only supported on Windows"
}

if ($LaunchOnly) {
  $installedExe = Find-InstalledWhisperingExe
  if (-not $installedExe) {
    throw "Installed Whispering.exe not found. Run 'make install-windows' first."
  }

  Write-Host "Launching installed Windows app: $installedExe"
  Start-Process -FilePath $installedExe | Out-Null
  exit 0
}

$rootDir = Split-Path -Parent $PSScriptRoot
$msiDir = Join-Path $rootDir "src-tauri\target\release\bundle\msi"

if (-not (Test-Path $msiDir)) {
  throw "Windows MSI directory not found: $msiDir. Run 'make release-windows' first."
}

if (-not $MsiPath) {
  $msi = Get-ChildItem -Path $msiDir -Filter *.msi -File | Sort-Object LastWriteTime -Descending | Select-Object -First 1
  if (-not $msi) {
    throw "Windows MSI not found under $msiDir. Run 'make release-windows' first."
  }

  $MsiPath = $msi.FullName
}

if (-not (Test-Path $MsiPath)) {
  throw "Windows MSI not found: $MsiPath"
}

Write-Host "Installing Windows MSI: $MsiPath"
$process = Start-Process -FilePath "msiexec.exe" -ArgumentList @("/i", $MsiPath, "/passive", "/norestart") -Wait -PassThru

if ($process.ExitCode -ne 0) {
  throw "MSI installation failed with exit code $($process.ExitCode)"
}

$installedExe = Find-InstalledWhisperingExe
if ($installedExe) {
  Write-Host "Installed Windows app: $installedExe"
  Write-Host "Launch it with 'make run-windows-app' or from the Start menu."
} else {
  Write-Host "MSI installation completed."
  Write-Host "Launch Whispering from the Start menu."
}
