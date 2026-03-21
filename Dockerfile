# Build stage
FROM rust:1.89-bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --release --features full --bin agnosai-server

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl libssl3 && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/agnosai-server /usr/local/bin/agnosai-server
EXPOSE 8080
ENV RUST_LOG=info
ENTRYPOINT ["agnosai-server"]
