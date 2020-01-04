# Builder ---------------------------------------------------------------------
FROM rust:1.40 AS builder
WORKDIR /usr/src/running-mate/
# Install Musl support
RUN rustup target add x86_64-unknown-linux-musl
RUN apt-get update && apt-get install -y musl-tools
# Initialize empty project
RUN USER=root cargo init
# Copy Cargo.toml & Cargo.lock for dependencies
COPY Cargo.* ./
# This is a dummy build to get the dependencies cached
RUN cargo build --target x86_64-unknown-linux-musl --release
# Copy over the code
COPY src/ src/
# Sleeping and touching before building is necessary so the timestamp of
# main.rs is not the same it was when we initialized the empty project for
# dependency caching
RUN sleep 1 && touch src/main.rs && \
  cargo build --target x86_64-unknown-linux-musl --release

# Final image -----------------------------------------------------------------
FROM alpine:3
COPY --from=builder \
  /usr/src/running-mate/target/x86_64-unknown-linux-musl/release/running-mate \
  /usr/local/bin/running-mate
CMD ["running-mate"]
