FROM rust:1-bookworm AS builder
WORKDIR /workspace

COPY . .
RUN cargo build --release -p wx-gateway

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 wx \
    && useradd --uid 10001 --gid wx --create-home --shell /usr/sbin/nologin wx \
    && mkdir -p /var/lib/rocksoul/receipts \
    && chown -R wx:wx /var/lib/rocksoul

COPY --from=builder /workspace/target/release/wx-gateway /usr/local/bin/wx-gateway

USER wx:wx
ENV WX_BIND_ADDR=0.0.0.0:8080
ENV WX_RECEIPT_DIR=/var/lib/rocksoul/receipts
EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/wx-gateway"]
