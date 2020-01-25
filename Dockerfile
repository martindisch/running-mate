# Builder ---------------------------------------------------------------------
FROM rust:1.40 AS builder
WORKDIR /usr/src/running-mate/
# Initialize empty project
RUN USER=root cargo init
# Copy Cargo.toml & Cargo.lock for dependencies
COPY Cargo.* ./
# This is a dummy build to get the dependencies cached
RUN cargo build --release
# Copy over the code
COPY src/ src/
# Sleeping and touching before building is necessary so the timestamp of
# main.rs is not the same it was when we initialized the empty project for
# dependency caching
RUN sleep 1 && touch src/main.rs && cargo build --release

# Final image -----------------------------------------------------------------
FROM debian:9-slim
ENV RUST_BACKTRACE=1
RUN apt-get update && apt-get -y install openssl ca-certificates
COPY --from=builder \
  /usr/src/running-mate/target/release/running-mate \
  /usr/local/bin/running-mate
CMD ["running-mate"]
