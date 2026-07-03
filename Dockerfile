FROM lukemathwalker/cargo-chef:latest-rust-1.95.0 AS chef
WORKDIR /app
RUN apt update && apt install lld clang -y


FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release

FROM chef AS migration
RUN cargo install --version='~0.8' sqlx-cli \
    --no-default-features \
    --features rustls,postgres
COPY migrations migrations
ENTRYPOINT ["sqlx", "migrate", "run"]

FROM debian:bookworm-slim AS runtime

WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/alleycat-rs alleycat-rs
COPY configuration configuration
ENV APP_ENVIRONMENT=production
ENTRYPOINT ["./alleycat-rs"]
