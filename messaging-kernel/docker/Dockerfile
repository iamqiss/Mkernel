# Multi-stage Dockerfile for Neo Messaging Kernel
# Optimized for production deployment with minimal image size

# Build stage
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /workspace

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY core/ ./core/
COPY compiler/ ./compiler/
COPY runtime/ ./runtime/
COPY tools/ ./tools/

# Build the project in release mode
RUN cargo build --release --workspace

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r neo && useradd -r -g neo neo

# Set working directory
WORKDIR /app

# Copy binaries from builder stage
COPY --from=builder /workspace/target/release/neo /usr/local/bin/
COPY --from=builder /workspace/target/release/neo-compiler /usr/local/bin/
COPY --from=builder /workspace/target/release/neo-cli /usr/local/bin/
COPY --from=builder /workspace/target/release/neo-debugger /usr/local/bin/

# Copy configuration files
COPY docker/neo.conf /etc/neo/neo.conf
COPY docker/logging.conf /etc/neo/logging.conf

# Create data directory
RUN mkdir -p /var/lib/neo && chown -R neo:neo /var/lib/neo

# Create log directory
RUN mkdir -p /var/log/neo && chown -R neo:neo /var/log/neo

# Switch to non-root user
USER neo

# Expose ports
EXPOSE 8080 9090 9091

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:9091/health || exit 1

# Default command
CMD ["neo", "--config", "/etc/neo/neo.conf"]