# syntax=docker/dockerfile:1

# ---- Builder ----
FROM rust:1.89-slim AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock* rust-toolchain.toml ./
COPY crates ./crates

RUN cargo build --release --bin scanengine-node

# ---- Runtime ----
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && rm -rf /var/lib/apt/lists/* \
    && useradd -u 10001 -m appuser

COPY --from=builder /app/target/release/scanengine-node /usr/local/bin/scanengine-node

USER 10001
EXPOSE 8081
ENV SCANENGINE_ADDR=0.0.0.0:8081

ENTRYPOINT ["/usr/local/bin/scanengine-node"]
CMD ["serve"]
