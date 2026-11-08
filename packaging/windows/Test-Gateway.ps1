$ErrorActionPreference = "Stop"
$baseUrl = "http://127.0.0.1:8080"
$keyPath = Join-Path $env:LOCALAPPDATA "ROCKSOUL-WX\gateway.key"
if (-not (Test-Path -LiteralPath $keyPath)) { throw "Local API key not found. Start ROCKSOUL-WX from the Start Menu first." }
$apiKey = ([System.IO.File]::ReadAllText($keyPath)).Trim()
if ($apiKey.Length -lt 32) { throw "The local API key is invalid." }

$health = Invoke-RestMethod -Method Get -Uri "$baseUrl/healthz" -TimeoutSec 5
if ($health.status -ne "ok") { throw "Gateway health check returned an unexpected status." }
Write-Host "PASS: gateway health endpoint is available." -ForegroundColor Green

$payload = @{ objective = "ROCKSOUL-WX Windows installer smoke test" } | ConvertTo-Json -Compress
$headers = @{ Authorization = "Bearer $apiKey" }
$plan = Invoke-RestMethod -Method Post -Uri "$baseUrl/v1/workloads/plan" -Headers $headers -ContentType "application/json" -Body $payload -TimeoutSec 10
if (-not $plan) { throw "The authenticated workload plan endpoint returned no response." }
Write-Host "PASS: authenticated API key and workload planning endpoint are working." -ForegroundColor Green
Write-Host "Execution plan:"
$plan | ConvertTo-Json -Depth 8
Write-Host ""
Write-Host "Installer smoke test completed. Model execution additionally requires a working local Ollama service and a downloaded model."
