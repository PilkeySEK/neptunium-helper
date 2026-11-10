FROM rust:latest AS build

WORKDIR /app

RUN apt-get install git

RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    --mount=type=bind,source=.git,target=.git \
    GIT_HASH=$(git rev-parse HEAD) \
    cargo build --locked --release --bin app && \
    cp /app/target/release/app /bin/app

FROM debian:trixie-slim AS runtime

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /bin/app ./app

COPY ./config.json ./config.json
# If .env doesn't exist that's fine, probably
COPY .env* .
# Replace with .env.example if not exists
RUN cp -u -p ./.env.example ./.env

CMD ["./app"]