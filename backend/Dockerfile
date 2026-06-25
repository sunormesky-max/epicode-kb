# Backend Dockerfile — multi-stage build
# Builder: rust:1.78, Runtime: debian:slim

FROM rust:1.78-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for better layer caching
COPY Cargo.toml ./
COPY src/ ./src/

# Build the binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/epicode-kb /app/epicode-kb

# Create data directories
RUN mkdir -p /app/data /app/models

# Expose port
EXPOSE 3000

# Environment
ENV EPICODE_KB_LISTEN_ADDR=0.0.0.0:3000
ENV EPICODE_KB_DATABASE_URL=/app/data/epicode_kb.db
ENV EPICODE_KB_TANTIVY_PATH=/app/data/tantivy
ENV EPICODE_KB_UPLOAD_DIR=/app/data/uploads
ENV RUST_LOG=epicode_kb=info

# Run
CMD ["/app/epicode-kb"]
