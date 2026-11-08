param(
    [Parameter(Mandatory = $true)]
    [string]$InstallerPath
)

$ErrorActionPreference = "Stop"
$installDirectory = Join-Path $env:LOCALAPPDATA "Programs\ROCKSOUL-WX"
$startScript = Join-Path $installDirectory "Start-Gateway.ps1"
$testScript = Join-Path $installDirectory "Test-Gateway.ps1"
$gatewayHost = $null
$installed = $false

try {
    if (-not (Test-Path -LiteralPath $InstallerPath)) {
        throw "Installer artifact not found: $InstallerPath"
    }

    $install = Start-Process -FilePath $InstallerPath -ArgumentList "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART" -Wait -PassThru
    if ($install.ExitCode -ne 0) {
        throw "Silent installer exited with code $($install.ExitCode)."
    }
    $installed = $true

    foreach ($requiredFile in @(
        (Join-Path $installDirectory "wx-gateway.exe"),
        $startScript,
        $testScript
    )) {
        if (-not (Test-Path -LiteralPath $requiredFile)) {
            throw "Installed file is missing: $requiredFile"
        }
    }

    $gatewayHost = Start-Process -FilePath "powershell.exe" -ArgumentList "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File `"$startScript`"" -WindowStyle Hidden -PassThru

    $deadline = (Get-Date).AddSeconds(45)
    $healthy = $false
    while ((Get-Date) -lt $deadline) {
        if ($gatewayHost.HasExited) {
            throw "Gateway launcher exited early with code $($gatewayHost.ExitCode)."
        }
        try {
            $health = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:8080/healthz" -TimeoutSec 2
            if ($health.status -eq "ok") {
                $healthy = $true
                break
            }
        } catch {
            Start-Sleep -Seconds 1
        }
    }
    if (-not $healthy) {
        throw "Installed gateway did not become healthy within 45 seconds."
    }

    $verification = Start-Process -FilePath "powershell.exe" -ArgumentList "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File `"$testScript`"" -Wait -PassThru -NoNewWindow
    if ($verification.ExitCode -ne 0) {
        throw "Installed gateway smoke test exited with code $($verification.ExitCode)."
    }

    Write-Host "PASS: installer installed, gateway started, health and authenticated planning passed."
}
finally {
    Get-Process -Name "wx-gateway" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

    if ($gatewayHost -and -not $gatewayHost.HasExited) {
        Stop-Process -Id $gatewayHost.Id -Force -ErrorAction SilentlyContinue
    }

    if ($installed -and (Test-Path -LiteralPath $installDirectory)) {
        $uninstaller = Get-ChildItem -LiteralPath $installDirectory -Filter "unins*.exe" | Select-Object -First 1
        if ($uninstaller) {
            $remove = Start-Process -FilePath $uninstaller.FullName -ArgumentList "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART" -Wait -PassThru
            if ($remove.ExitCode -ne 0) {
                throw "Silent uninstaller exited with code $($remove.ExitCode)."
            }
        }
    }
}
