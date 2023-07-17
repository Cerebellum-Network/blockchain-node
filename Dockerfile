FROM phusion/baseimage:0.11 as builder
LABEL maintainer="team@cere.network"
LABEL description="This is the build stage to create the binary."
ARG PROFILE=release
WORKDIR /cerenetwork
COPY . /cerenetwork

RUN apt-get update && \
	  apt-get upgrade -y && \
	  apt-get install -y cmake pkg-config libssl-dev git clang unzip

# Installation script is taken from https://grpc.io/docs/protoc-installation/
ENV PROTOC_VERSION=3.15.8
RUN PB_REL="https://github.com/protocolbuffers/protobuf/releases" && \
    curl -LO $PB_REL/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-x86_64.zip && \
    mkdir -p /usr/local/protoc && \
    unzip protoc-${PROTOC_VERSION}-linux-x86_64.zip -d /usr/local/protoc && \
    chmod +x /usr/local/protoc/bin/protoc && \
    ln -s /usr/local/protoc/bin/protoc /usr/local/bin

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
    export PATH=$PATH:$HOME/.cargo/bin && \
    scripts/init.sh && \
    cargo build --$PROFILE

# ===== SECOND STAGE ======
FROM phusion/baseimage:0.11
LABEL maintainer="team@cere.network"
LABEL description="This is the optimization to create a small image."
ARG PROFILE=release
COPY --from=builder /cerenetwork/target/$PROFILE/cere /usr/local/bin
COPY --from=builder /cerenetwork/target/release/wbuild/cere-runtime /home/cere/cere-runtime-artifacts
COPY --from=builder /cerenetwork/target/release/wbuild/cere-dev-runtime /home/cere/cere-dev-runtime-artifacts

RUN mv /usr/share/ca* /tmp && \
    rm -rf /usr/share/*  && \
    mv /tmp/ca-certificates /usr/share/ && \
    rm -rf /usr/lib/python* && \
    useradd -m -u 1000 -U -s /bin/sh -d /cerenetwork cerenetwork && \
    mkdir -p /cerenetwork/.local/share/cerenetwork && \
    mkdir -p /cerenetwork/.local/share/cere && \
    chown -R cerenetwork:cerenetwork /cerenetwork/.local && \
    ln -s /cerenetwork/.local/share/cere /data && \
    rm -rf /usr/bin /usr/sbin

USER cerenetwork
EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]

CMD ["/usr/local/bin/cere"]
