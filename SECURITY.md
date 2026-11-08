# Security Policy

## Supported deployment scope

The current release candidate is intended for controlled, single-operator deployments behind a TLS reverse proxy. It is not a multi-tenant SaaS service and is not security-certified.

## Reporting a vulnerability

Do not open a public issue containing exploit details, credentials, personal data, or sensitive workload content. Use GitHub's private vulnerability reporting for this repository when enabled, or contact the repository maintainers through a private channel.

Include the affected version/commit, deployment mode, impact, and a minimal reproduction that does not expose secrets or customer data.

## Secret handling

- Never commit gateway/provider credentials or production configuration files.
- Use secret files or the host's secret manager and restrict file permissions.
- Rotate credentials that may have been exposed.
- Keep public-cloud provider endpoints on HTTPS.
- Keep the gateway behind a TLS reverse proxy and do not publish port 8080 directly.

## Known security limitations

The gateway currently uses a single shared API key, in-memory per-key rate limiting, local JSON receipts, and operator-maintained provider metadata. Tenant isolation, distributed rate limiting, centralized secret management, tamper-evident receipts, and hard budget reservations are not implemented.
