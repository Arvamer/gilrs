FROM debian:jessie

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && \
    apt-get install \
       ca-certificates \
       curl \
       gcc \
       libc6-dev \
       pkg-config \
       libudev-dev \
       git \
       binutils-mingw-w64-x86-64 \
       gcc-mingw-w64-x86-64 \
       -qqy \
       --no-install-recommends \
    && rm -rf /var/lib/apt/lists/*

ENV RUST_VERSION=1.12.0

ENV RUST_URL_BASE=https://static.rust-lang.org/dist
ENV RUST_ARCHIVE=rust-$RUST_VERSION-x86_64-unknown-linux-gnu.tar.gz
ENV RUST_ARCHIVE_URL=$RUST_URL_BASE/$RUST_ARCHIVE
ENV RUST_WIN_STDLIB=rust-std-$RUST_VERSION-x86_64-pc-windows-gnu.tar.gz
ENV RUST_WIN_STDLIB_URL=$RUST_URL_BASE/$RUST_WIN_STDLIB

RUN mkdir -p /rust
WORKDIR /rust

RUN curl -fsOSL $RUST_ARCHIVE_URL \
    && curl -s $RUST_ARCHIVE_URL.sha256 | sha256sum -c - \
    && tar -C /rust -xzf $RUST_ARCHIVE --strip-components=1 \
    && rm $RUST_ARCHIVE \
    && ./install.sh --without=rust-docs \
    && curl -fsOSL $RUST_WIN_STDLIB_URL \
    && curl -s $RUST_WIN_STDLIB_URL.sha256 | sha256sum -c - \
    && tar -C /rust -xzf $RUST_WIN_STDLIB --strip-components=1 \
    && rm $RUST_WIN_STDLIB \
    && ./install.sh \
    && rm -rf * \
    && mkdir .cargo

COPY config .cargo/config
