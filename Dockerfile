FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin app -F docker

# We do not need the Rust toolchain to run the binary!
FROM debian:trixie-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates
# COPY your_certificate.crt /usr/local/share/ca-certificates/
RUN update-ca-certificates
WORKDIR /app
COPY config.json /etc/config.json
COPY .env /app/.env
COPY --from=builder /app/target/release/app /usr/local/bin
ENTRYPOINT ["/usr/local/bin/app"]