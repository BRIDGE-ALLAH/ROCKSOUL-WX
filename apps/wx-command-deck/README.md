# ROCKSOUL-WX Command Deck

The Dioxus web UI is a browser-based operator console for the workload API.

## Run locally

Start the gateway on `127.0.0.1:8080`, then:

```bash
dx serve --package wx-command-deck --platform web --port 8081
```

The UI defaults to `http://127.0.0.1:8080`. Configure `WX_CORS_ORIGINS` on the gateway if the UI is served from another origin.

For a remote gateway, set `WX_API_KEY` on the server and enter the corresponding Bearer key in the UI. The key is held in the page's memory only; do not use the browser console on shared machines.

## Current capabilities

- Submit a workload contract to `POST /v1/workloads/execute`.
- Choose the data classification and a single execution location.
- Explicitly allow or deny raw-data egress.
- Display the execution plan, model output, acceptance checks, and receipt.

The UI does not call provider endpoints directly and does not contain provider credentials. Streaming, user management, persistent browser settings, and live aggregate metrics are not part of this MVP.
