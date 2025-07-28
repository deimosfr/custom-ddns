# Stage 1: Build stage
FROM rust:1.87.0-slim AS builder

RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . ./
RUN cargo build --release

# Stage 2: Runtime stage
FROM debian:stable-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false app

WORKDIR /app
COPY --from=builder /app/target/release/custom-ddns /app/custom-ddns
RUN chown app:app /app/custom-ddns
USER app

ENTRYPOINT ["/app/custom-ddns"]