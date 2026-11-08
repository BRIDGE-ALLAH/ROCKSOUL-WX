# ROCKSOUL-WX for Windows

## What this installer provides

The x64 installer installs the native Rust wx-gateway.exe and PowerShell helper scripts. It is intended for local Windows testing of the gateway API. It does not install Ollama, download an AI model, or install a native desktop Command Deck.

Supported baseline: Windows 10/11 x64 with Windows PowerShell 5.1 or later.

## Install and start

1. Download ROCKSOUL-WX-Setup-<version>.exe from the GitHub Release page and run it.
2. Open ROCKSOUL-WX -> Start Local Gateway from the Start Menu.
3. Keep the console window open. The gateway listens only on http://127.0.0.1:8080.
4. Open a second PowerShell window and run the installed Test-Gateway.ps1 script, or open Verify Local Gateway from the Start Menu.

The installer smoke test checks /healthz and makes an authenticated request to /v1/workloads/plan. It does not send a request to an AI provider.

## Enable local model execution

The default provider is an Ollama-compatible service at http://127.0.0.1:11434 using model llama3.2.

Install and start Ollama separately, then download the model:

~~~powershell
ollama pull llama3.2
~~~

Restart the ROCKSOUL-WX gateway after Ollama is ready. Provider execution will fail until the local provider is available.

## Local credentials and data

- A 256-bit random gateway key is created on first start at %LOCALAPPDATA%\ROCKSOUL-WX\gateway.key.
- Both keys are kept outside the installation directory and are reused on later starts.
- Execution receipt metadata is stored under %LOCALAPPDATA%\ROCKSOUL-WX\receipts.
- Receipt HMAC detects content alteration when the key is protected, but does not detect deletion or rollback of old files and is not a distributed ledger. The installer binds the API to loopback only. Do not change this to a LAN/public bind without configuring a strong key, firewall rules, TLS termination, and a reviewed deployment architecture.
- To retrieve the local API key for a separate client, run: Get-Content "$env:LOCALAPPDATA\ROCKSOUL-WX\gateway.key".

Stop the gateway with Ctrl+C in its console window. Uninstalling the application does not automatically remove user data or the local API key.

## Release integrity and signing

Each release publishes a SHA-256 checksum next to the installer. The installer is currently NOT Authenticode-signed because no code-signing certificate has been configured for this repository. Windows SmartScreen may therefore display an unknown-publisher warning. Verify the checksum against the .sha256 file before running the installer.

This is a single-operator local test package, not a multi-tenant service or a production desktop GUI.
