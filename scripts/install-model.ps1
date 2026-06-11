param(
  [string]$Platform = "windows",
  [string]$ModelName = "ggml-medium.bin",
  [string]$ModelUrl = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin"
)

$Platform = $Platform.ToLowerInvariant()
if ($Platform -ne "windows") {
  Write-Error "install-model.ps1 only supports windows"
  exit 1
}

$modelsDir = Join-Path $HOME ".whispering\models"
$modelPath = Join-Path $modelsDir $ModelName

Write-Host "Detected OS: $Platform"
New-Item -ItemType Directory -Force -Path $modelsDir | Out-Null

if (Test-Path $modelPath) {
  Write-Host "Model already installed at $modelPath"
  exit 0
}

Write-Host "Downloading $ModelName to $modelPath"
Invoke-WebRequest -Uri $ModelUrl -OutFile $modelPath
Write-Host "Model installed to $modelPath"
