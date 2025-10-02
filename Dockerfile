FROM phusion/baseimage:jammy-1.0.1 as builder
LABEL maintainer="team@cere.network"
LABEL description="Build stage to create the binaries and wasm artifacts."

ARG PROFILE=release
ARG GH_READ_TOKEN

WORKDIR /cerenetwork

# Install required packages in a single layer and clean apt lists to reduce image size
RUN apt-get -qq update && \
    DEBIAN_FRONTEND=noninteractive apt-get -qq install -y --no-install-recommends \
        ca-certificates \
        clang \
        cmake \
        git \
        libpq-dev \
        libssl-dev \
        pkg-config \
        unzip \
        wget \
        curl \
    && rm -rf /var/lib/apt/lists/*

# Install sccache for faster Rust builds
ENV SCCACHE_VERSION=0.5.4 \
    RUSTC_WRAPPER=/usr/local/bin/sccache \
    SCCACHE_DIR=/opt/sccache
RUN wget -q https://github.com/mozilla/sccache/releases/download/v${SCCACHE_VERSION}/sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz \
        -O - | tar -xz && \
    mv sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl/sccache /usr/local/bin/sccache && \
    chmod +x /usr/local/bin/sccache && \
    rm -rf sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl

# Create unprivileged build user
RUN useradd -m -u 1001 builder
USER builder

# Install protoc (required by the build) into user space
ENV PROTOC_VERSION=3.15.8
RUN PB_REL="https://github.com/protocolbuffers/protobuf/releases" && \
    curl -sLO $PB_REL/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-x86_64.zip && \
    mkdir -p /home/builder/protoc && \
    unzip -q protoc-${PROTOC_VERSION}-linux-x86_64.zip -d /home/builder/protoc && \
    ln -s /home/builder/protoc/bin/protoc /home/builder/.local/bin/protoc && \
    rm protoc-${PROTOC_VERSION}-linux-x86_64.zip
ENV PATH=/home/builder/.local/bin:$PATH

# Configure Git to use token for private dependencies
RUN git config --global url."https://${GH_READ_TOKEN}:x-oauth-basic@github.com/".insteadOf "https://github.com/"

# Install Rust toolchain
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y
ENV PATH=/home/builder/.cargo/bin:$PATH \
    CARGO_TERM_COLOR=always

# Copy sources (consider adding a .dockerignore to avoid copying unnecessary files)
COPY --chown=builder:builder . /cerenetwork

# Build with BuildKit caches for cargo registry, git, target, and sccache
RUN --mount=type=cache,target=/opt/sccache \
    --mount=type=cache,target=/home/builder/.cargo/registry \
    --mount=type=cache,target=/home/builder/.cargo/git \
    --mount=type=cache,target=/cerenetwork/target \
    scripts/init.sh && \
    cargo build --locked --${PROFILE} --features on-chain-release-build

# ---------- Runtime image ----------
FROM phusion/baseimage:jammy-1.0.1 as runtime
LABEL maintainer="team@cere.network"
LABEL description="Optimized runtime image."

ARG PROFILE=release

# Copy binaries and wasm artifacts from the builder stage
COPY --from=builder /cerenetwork/target/$PROFILE/cere /usr/local/bin
COPY --from=builder /cerenetwork/target/$PROFILE/wbuild/cere-runtime /home/cere/cere-runtime-artifacts
COPY --from=builder /cerenetwork/target/$PROFILE/wbuild/cere-dev-runtime /home/cere/cere-dev-runtime-artifacts

# Minimize the image and create unprivileged runtime user
RUN mv /usr/share/ca* /tmp && \
    rm -rf /usr/share/* && \
    mv /tmp/ca-certificates /usr/share/ && \
    rm -rf /usr/lib/python* && \
    useradd -m -u 1000 -U -s /bin/sh -d /cerenetwork cerenetwork && \
    mkdir -p /cerenetwork/.local/share/cerenetwork /cerenetwork/.local/share/cere && \
    chown -R cerenetwork:cerenetwork /cerenetwork/.local && \
    ln -s /cerenetwork/.local/share/cere /data && \
    mv -t /usr/local/bin /usr/bin/bash /usr/bin/sh && \
    rm -rf /usr/bin /usr/sbin

USER cerenetwork
EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]
CMD ["/usr/local/bin/cere"]

# ---------- Artifacts stage (for direct export via buildx --output) ----------
FROM scratch as artifacts
ARG PROFILE=release
ARG SHORT_SHA
# Place wasm files under /out so buildx can export them locally without docker cp
COPY --from=builder /cerenetwork/target/$PROFILE/wbuild/cere-runtime/cere_runtime.compact.compressed.wasm /out/cere_runtime.compact.compressed.${SHORT_SHA}.wasm
COPY --from=builder /cerenetwork/target/$PROFILE/wbuild/cere-dev-runtime/cere_dev_runtime.compact.compressed.wasm /out/cere_dev_runtime.compact.compressed.${SHORT_SHA}.wasm
