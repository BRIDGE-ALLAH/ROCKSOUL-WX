$ErrorActionPreference = "Stop"

$gatewayExe = Join-Path $PSScriptRoot "wx-gateway.exe"
if (-not (Test-Path -LiteralPath $gatewayExe)) {
    throw "wx-gateway.exe was not found beside this script. Re-run the ROCKSOUL-WX installer."
}
$dataDir = Join-Path $env:LOCALAPPDATA "ROCKSOUL-WX"
$receiptDir = Join-Path $dataDir "receipts"
$keyPath = Join-Path $dataDir "gateway.key"
New-Item -ItemType Directory -Force -Path $dataDir, $receiptDir | Out-Null

if (-not (Test-Path -LiteralPath $keyPath)) {
    $bytes = New-Object byte[] 32
    $rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
    try { $rng.GetBytes($bytes) } finally { $rng.Dispose() }
    $key = [BitConverter]::ToString($bytes).Replace("-", "").ToLowerInvariant()
    [System.IO.File]::WriteAllText($keyPath, $key, [System.Text.Encoding]::ASCII)
}
$apiKey = ([System.IO.File]::ReadAllText($keyPath)).Trim()
if ($apiKey.Length -lt 32) { throw "The local gateway key is invalid. Delete '$keyPath' and restart to regenerate it." }

# Local-only defaults: this test installer never exposes the gateway to the LAN.
$env:WX_BIND_ADDR = "127.0.0.1:8080"
$env:WX_API_KEY = $apiKey
$env:WX_RATE_LIMIT_PER_MINUTE = "120"
$env:WX_ROUTE_OBJECTIVE = "balanced"
$env:WX_RECEIPT_DIR = $receiptDir
$env:WX_UPSTREAM_NAME = "ollama"
$env:WX_UPSTREAM_BASE_URL = "http://127.0.0.1:11434/v1"
$env:WX_UPSTREAM_MODEL = "llama3.2"
$env:WX_UPSTREAM_LOCATION = "local"
$env:WX_UPSTREAM_TIMEOUT_SECONDS = "60"
$env:WX_CORS_ORIGINS = "http://localhost:8081,http://127.0.0.1:8081"

Write-Host ""
Write-Host "ROCKSOUL-WX local gateway" -ForegroundColor Cyan
Write-Host "Listening: http://127.0.0.1:8080"
Write-Host "API key file: $keyPath"
Write-Host "Receipts: $receiptDir"
Write-Host "Provider: Ollama at http://127.0.0.1:11434 (model llama3.2)"
Write-Host "The API key is stored for the current Windows user only."
Write-Host "Use Test-Gateway.ps1 in another PowerShell window to verify the installation."
Write-Host "Press Ctrl+C in this window to stop the gateway."
Write-Host ""
& $gatewayExe
$exitCode = $LASTEXITCODE
if ($exitCode -ne 0) { Write-Error "ROCKSOUL-WX gateway exited with code $exitCode." }
exit $exitCode
