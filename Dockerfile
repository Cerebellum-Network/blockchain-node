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

# Create non-privileged user for building
RUN useradd -m -u 1001 builder

# Change ownership of the workspace to builder user
RUN chown -R builder:builder /cerenetwork

# Installation script is taken from https://grpc.io/docs/protoc-installation/
ENV PROTOC_VERSION=3.15.8
RUN PB_REL="https://github.com/protocolbuffers/protobuf/releases" && \
    curl -sLO $PB_REL/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-x86_64.zip && \
    mkdir -p /usr/local/protoc && \
    unzip protoc-${PROTOC_VERSION}-linux-x86_64.zip -d /usr/local/protoc && \
    chmod +x /usr/local/protoc/bin/protoc && \
    ln -s /usr/local/protoc/bin/protoc /usr/local/bin/protoc && \
    chmod +x /usr/local/bin/protoc && \
    chown -R builder:builder /usr/local/protoc

# GitHub token for private repository access (temporary during build)
ARG GH_READ_TOKEN

# Switch to builder user and configure Git
USER builder
RUN git config --global url."https://${GH_READ_TOKEN}:x-oauth-basic@github.com/".insteadOf "https://github.com/"

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
    export PATH=$PATH:$HOME/.cargo/bin && \
    scripts/init.sh && \
    cargo build --locked --$PROFILE --features on-chain-release-build

# ===== SECOND STAGE ======
FROM phusion/baseimage:jammy-1.0.1
LABEL maintainer="team@cere.network"
LABEL description="This is the optimization to create a small image."
ARG PROFILE=release

# Copy binaries from builder stage
COPY --from=builder /cerenetwork/target/$PROFILE/cere /usr/local/bin
COPY --from=builder /cerenetwork/target/$PROFILE/wbuild/cere-runtime /home/cere/cere-runtime-artifacts
COPY --from=builder /cerenetwork/target/$PROFILE/wbuild/cere-dev-runtime /home/cere/cere-dev-runtime-artifacts

# Optimize image size and create non-privileged user
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
