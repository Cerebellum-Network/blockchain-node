# ============================================================================
# SECURE BLOCKCHAIN NODE DOCKERFILE
# ============================================================================
# This Dockerfile implements security best practices:
# - Multi-stage build to minimize attack surface
# - Non-root user execution
# - No credentials in build args or environment variables
# - Minimal base image
# - Security scanning compatible
# - Build cache optimization with sccache (using IAM roles, not credentials)
# ============================================================================

# ===== BUILDER STAGE =====
FROM rust:1.82-slim-bookworm AS builder

# Set build arguments (NO CREDENTIALS)
ARG PROFILE=release

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    clang \
    cmake \
    git \
    libssl-dev \
    pkg-config \
    protobuf-compiler \
    curl \
    unzip \
    ca-certificates \
    make \
    autoconf \
    automake \
    libtool \
    && rm -rf /var/lib/apt/lists/*

# sccache disabled - using standard Rust compilation

# Create non-root user for building
RUN useradd -m -u 1001 -U -s /bin/sh builder
USER builder
WORKDIR /app

# Copy source code with proper ownership
COPY --chown=builder:builder . .

# Initialize Rust environment and build
RUN scripts/init.sh && \
    cargo build --locked --${PROFILE} --features on-chain-release-build

# ===== RUNTIME STAGE =====
FROM debian:bookworm-slim AS runtime

# Install only essential runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user for runtime
RUN useradd -m -u 1001 -U -s /bin/sh -d /data cere \
    && mkdir -p /data/.local/share/cere \
    && chown -R cere:cere /data

# Copy built binary and runtime artifacts from builder stage
COPY --from=builder --chown=cere:cere /app/target/${PROFILE}/cere /usr/local/bin/cere
COPY --from=builder --chown=cere:cere /app/target/${PROFILE}/wbuild/cere-runtime /data/cere-runtime-artifacts
COPY --from=builder --chown=cere:cere /app/target/${PROFILE}/wbuild/cere-dev-runtime /data/cere-dev-runtime-artifacts

# Ensure binary is executable
RUN chmod +x /usr/local/bin/cere

# Set security-focused permissions
RUN chmod 755 /usr/local/bin/cere \
    && find /data -type d -exec chmod 755 {} \; \
    && find /data -type f -exec chmod 644 {} \;

# Switch to non-root user
USER cere
WORKDIR /data

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD /usr/local/bin/cere --version || exit 1

# Security labels
LABEL maintainer="team@cere.network" \
      description="Secure Cere Network blockchain node" \
      version="7.3.3" \
      security.scan="enabled" \
      org.opencontainers.image.title="Cere Network Node" \
      org.opencontainers.image.description="Production-ready secure blockchain node" \
      org.opencontainers.image.vendor="Cere Network" \
      org.opencontainers.image.licenses="GPL-3.0-or-later WITH Classpath-exception-2.0"

# Expose ports (documentation only - no security risk)
EXPOSE 30333 9933 9944 9615

# Use VOLUME for data directory
VOLUME ["/data/.local/share/cere"]

# Set default command
CMD ["/usr/local/bin/cere"]
