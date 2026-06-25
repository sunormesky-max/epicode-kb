# Backend Dockerfile — multi-stage build, non-root runtime, healthcheck.
# Build features can be controlled via BUILD_FEATURES (default: full).

FROM rust:1.78-slim AS builder

WORKDIR /app

# Install build dependencies.
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for better layer caching.
COPY Cargo.toml ./
COPY src/ ./src/

ARG BUILD_FEATURES=full
RUN cargo build --release --features ${BUILD_FEATURES}

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies.
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user.
RUN groupadd -r epicode && useradd -r -g epicode -d /app epicode

# Copy binary.
COPY --from=builder /app/target/release/epicode-kb /app/epicode-kb

# Create data / model directories and set ownership.
RUN mkdir -p /app/data /app/models /app/data/uploads /app/data/tantivy && \
    chown -R epicode:epicode /app

USER epicode

# Expose port.
EXPOSE 3000

# Environment.
ENV EPICODE_KB_LISTEN_ADDR=0.0.0.0:3000
ENV EPICODE_KB_DATABASE_URL=/app/data/epicode_kb.db
ENV EPICODE_KB_TANTIVY_PATH=/app/data/tantivy
ENV EPICODE_KB_UPLOAD_DIR=/app/data/uploads
ENV RUST_LOG=epicode_kb=info

# Healthcheck.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -fs http://localhost:3000/api/v1/system/health || exit 1

# Run.
CMD ["/app/epicode-kb"]
