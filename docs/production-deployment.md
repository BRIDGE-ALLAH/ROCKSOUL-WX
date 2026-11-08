# Production Deployment Runbook

This release path targets a **single-operator / single-trust-domain deployment**. It is not a multi-tenant SaaS deployment. The gateway still has one shared API credential, local JSON receipts, and no tenant identity or per-tenant isolation.

## 1. Infrastructure prerequisites

- A Linux host with Docker Engine and the Docker Compose v2 plugin.
- A DNS name for the API (for example, `api.example.com`) pointing to the host.
- Inbound TCP ports 80 and 443 (and optionally UDP 443) allowed to the host.
- A hosted Command Deck frontend. This repository's production Compose stack publishes the API, not the frontend.
- An operator-controlled OpenAI-compatible inference endpoint reachable from the gateway container.
- A provider account and API key, where applicable. Non-local provider URLs (private and public cloud) must use HTTPS.

Do not expose port 8080 directly. Caddy is the only service with published host ports and terminates HTTPS automatically. The gateway is not host-published; it shares an application bridge with Caddy and has outbound connectivity for configured provider calls.

## 2. Prepare secrets

From the repository root:

```bash
mkdir -p secrets config
umask 077
openssl rand -hex 32 > secrets/gateway_api_key
: > secrets/primary_provider_api_key
chmod 600 secrets/gateway_api_key secrets/primary_provider_api_key
# The gateway container runs as uid/gid 10001; allow only that identity to read.
sudo chown 10001:10001 secrets/gateway_api_key secrets/primary_provider_api_key
sudo chmod 0400 secrets/gateway_api_key secrets/primary_provider_api_key
```

Place the real provider key into `secrets/primary_provider_api_key` using your secret-delivery process. Do not commit secret files or paste credentials into issues, chat, or source files.

The gateway reads `WX_API_KEY_FILE` and provider `*_FILE` environment variables at process startup. Compose file-source secrets are read-only host-file mounts, so the files must be readable by container uid/gid 10001; the commands above set this explicitly. Secret rotation requires updating the file and recreating the gateway container.

## 3. Configure providers

Copy the catalog template:

```bash
cp config/providers.production.example.json config/providers.production.json
```

Edit each profile with the actual private inference URL, approved model ID, location, timeout, and secret environment variable name. Set `enabled` to `true` only after testing the endpoint. Public-cloud profiles must use HTTPS. Confidential and restricted workloads are never eligible for public-cloud profiles; public-cloud use also requires explicit workload egress permission.

`quality_score` (0–100) and `p50_latency_ms` are operator-maintained routing inputs, not benchmark results supplied by ROCKSOUL-WX. Populate them from your own repeatable evaluation. Set both per-1K-token price fields from the provider's current official pricing schedule. The gateway records an estimated charge only when both prices and provider-reported token usage are available. Estimates can be stale and are not invoices or spend caps.

Routing objectives:

- `balanced`: weighted rank of quality, latency, and known price.
- `quality`: highest operator-supplied quality score first.
- `cost`: lowest sum of input/output price rates first; profiles with unknown rates rank last.
- `latency`: lowest operator-supplied p50 latency first.

The planner's trust boundary takes precedence over these objectives: local is preferred over private cloud, and private cloud over public cloud when the contract explicitly permits the corresponding fallback. There is no automatic retry/failover after a provider call fails in this release.

## 4. Configure the host

```bash
cp .env.production.example .env.production
```

Set `WX_IMAGE_TAG` to a tag actually published in GHCR. Set `WX_DOMAIN`, `ACME_EMAIL`, and `WX_CORS_ORIGINS` to real values. The CORS origin must match the hosted Command Deck exactly, including scheme and port.

Create the key file and provider catalog before starting:

```bash
docker compose --env-file .env.production -f compose.production.yaml config --quiet
docker compose --env-file .env.production -f compose.production.yaml pull
docker compose --env-file .env.production -f compose.production.yaml up -d
```

Caddy obtains and renews TLS certificates. Check the public health endpoint:

```bash
curl --fail https://$WX_DOMAIN/healthz
```

The health response reports service status, configured provider count, and routing objective; it intentionally omits models, URLs, and credentials.

Configure the Command Deck's gateway URL as `https://$WX_DOMAIN`. Keep the gateway API key in the frontend's runtime input only; do not bake it into a static JavaScript bundle.

## 5. Operations

- Monitor container restarts, provider errors, 429 rate-limit responses, disk capacity, Caddy certificate renewal, and provider billing independently.
- Back up the `rocksoul_wx_receipts` and Caddy volumes. Receipts are local JSON metadata files; they are not a distributed or tamper-evident audit ledger.
- Rotate gateway and provider credentials through the secret file paths and recreate the gateway container.
- Review provider price schedules and re-measure quality/latency inputs whenever providers or models change.
- Roll back by changing `WX_IMAGE_TAG` to the previous known-good image and recreating the stack.
- Keep image tags immutable and retain the matching SBOM/provenance/signature from the release workflow.

The in-process rate limit is per gateway credential and per 60-second window. A single shared key means all callers share one bucket. It is not a substitute for tenant-level quotas, a distributed limiter, or identity-based access control.

## 6. Release gates and limitations

Before a public or multi-customer launch, complete and independently review:

1. Real provider integration and model-specific quality/latency benchmarks.
2. Tenant identity, isolated credentials, per-tenant quotas, and authorization.
3. A production receipt store with access controls, retention, backups, and integrity protection.
4. Hard budget reservation/settlement. Current cost is a post-execution estimate; `max_cost_microusd` remains rejected.
5. Human approval workflow for contracts that require it.
6. External monitoring/alerting, incident response, key rotation automation, vulnerability scanning, and a recovery exercise.
7. A deployed frontend and end-to-end browser test over the real domain.

This runbook makes the service deployable behind HTTPS for a controlled single-operator environment. It does not claim security certification, multi-tenant readiness, or that a real cloud provider has been tested. A domain, host, DNS, real credentials, and operational owner must be supplied and verified separately.
