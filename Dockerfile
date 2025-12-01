FROM phusion/baseimage:jammy-1.0.1 as builder
LABEL maintainer="team@cere.network"
LABEL description="This is the build stage to create the binary."
ARG PROFILE=release
WORKDIR /cerenetwork
COPY . /cerenetwork

RUN apt-get -qq update && \
	  apt-get -qq install -y \
      clang \
      cmake \
      git \
      libpq-dev \
      libssl-dev \
      pkg-config \
      unzip \
      wget

# Configure sccache
ENV SCCACHE_VERSION=0.5.4
RUN wget -q https://github.com/mozilla/sccache/releases/download/v${SCCACHE_VERSION}/sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz \
      -O - | tar -xz \
    && mv sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl/sccache /usr/local/bin/sccache \
    && chmod +x /usr/local/bin/sccache \
    && rm -rf sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl
ENV RUSTC_WRAPPER=/usr/local/bin/sccache

# Installation script is taken from https://grpc.io/docs/protoc-installation/
ENV PROTOC_VERSION=3.15.8
RUN PB_REL="https://github.com/protocolbuffers/protobuf/releases" && \
    curl -sLO $PB_REL/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-x86_64.zip && \
    mkdir -p /usr/local/protoc && \
    unzip protoc-${PROTOC_VERSION}-linux-x86_64.zip -d /usr/local/protoc && \
    chmod +x /usr/local/protoc/bin/protoc && \
    ln -s /usr/local/protoc/bin/protoc /usr/local/bin

ARG AWS_ACCESS_KEY_ID
ARG AWS_SECRET_ACCESS_KEY
ARG AWS_SESSION_TOKEN
ARG SCCACHE_REGION=us-west-2
ARG SCCACHE_BUCKET=cere-blockchain-sccache
ENV \
  AWS_ACCESS_KEY_ID=$AWS_ACCESS_KEY_ID \
  AWS_SECRET_ACCESS_KEY=$AWS_SECRET_ACCESS_KEY \
  AWS_SESSION_TOKEN=$AWS_SESSION_TOKEN \
  AWS_REGION=$SCCACHE_REGION \
  SCCACHE_REGION=$SCCACHE_REGION \
  SCCACHE_BUCKET=$SCCACHE_BUCKET \
  SCCACHE_S3_USE_SSL=true

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
    export PATH=$PATH:$HOME/.cargo/bin && \
    scripts/init.sh && \
    cargo build  --$PROFILE  --features on-chain-release-build

# Aggressively clean up unnecessary files to save disk space and prevent runner resource exhaustion
# Remove debug builds, incremental compilation artifacts, and other intermediate files
# Keep only the release binary and final WASM runtime artifacts
RUN rm -rf /cerenetwork/target/debug && \
    rm -rf /cerenetwork/target/$PROFILE/incremental && \
    rm -rf /cerenetwork/target/$PROFILE/build && \
    find /cerenetwork/target/$PROFILE/deps -name "*.rlib" -delete 2>/dev/null || true && \
    find /cerenetwork/target/$PROFILE/deps -name "*.rmeta" -delete 2>/dev/null || true && \
    find /cerenetwork/target/$PROFILE/deps -name "*.d" -delete 2>/dev/null || true && \
    # Aggressively clean up wbuild directories - keep only the final WASM files
    # This is critical to prevent runner resource exhaustion during COPY operations
    find /cerenetwork/target/$PROFILE/wbuild/cere-runtime -type f ! -name "cere_runtime.compact.compressed.wasm" -delete 2>/dev/null || true && \
    find /cerenetwork/target/$PROFILE/wbuild/cere-dev-runtime -type f ! -name "cere_dev_runtime.compact.compressed.wasm" -delete 2>/dev/null || true && \
    find /cerenetwork/target/$PROFILE/wbuild -type d -empty -delete 2>/dev/null || true && \
    # Clean up Rust toolchain cache
    rm -rf ~/.cargo/registry/cache 2>/dev/null || true && \
    # Clean up apt cache
    apt-get clean && rm -rf /var/lib/apt/lists/* && \
    # Clean up temporary files
    rm -rf /tmp/* /var/tmp/* && \
    # Verify WASM files exist before proceeding
    test -f /cerenetwork/target/$PROFILE/wbuild/cere-runtime/cere_runtime.compact.compressed.wasm && \
    test -f /cerenetwork/target/$PROFILE/wbuild/cere-dev-runtime/cere_dev_runtime.compact.compressed.wasm

# ===== SECOND STAGE ======
FROM phusion/baseimage:jammy-1.0.1
LABEL maintainer="team@cere.network"
LABEL description="This is the optimization to create a small image."
ARG PROFILE=release
COPY --from=builder /cerenetwork/target/$PROFILE/cere /usr/local/bin

# Copy only the necessary WASM files instead of entire directories for faster builds
RUN mkdir -p /home/cere/cere-runtime-artifacts /home/cere/cere-dev-runtime-artifacts
COPY --from=builder /cerenetwork/target/$PROFILE/wbuild/cere-runtime/cere_runtime.compact.compressed.wasm /home/cere/cere-runtime-artifacts/
COPY --from=builder /cerenetwork/target/$PROFILE/wbuild/cere-dev-runtime/cere_dev_runtime.compact.compressed.wasm /home/cere/cere-dev-runtime-artifacts/

RUN mv /usr/share/ca* /tmp && \
    rm -rf /usr/share/*  && \
    mv /tmp/ca-certificates /usr/share/ && \
    rm -rf /usr/lib/python* && \
    useradd -m -u 1000 -U -s /bin/sh -d /cerenetwork cerenetwork && \
    mkdir -p /cerenetwork/.local/share/cerenetwork && \
    mkdir -p /cerenetwork/.local/share/cere && \
    chown -R cerenetwork:cerenetwork /cerenetwork/.local && \
    ln -s /cerenetwork/.local/share/cere /data && \
    mv -t /usr/local/bin /usr/bin/bash /usr/bin/sh && \
    rm -rf /usr/bin /usr/sbin

USER cerenetwork
EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]

CMD ["/usr/local/bin/cere"]
