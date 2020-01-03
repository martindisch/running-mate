# Builder ---------------------------------------------------------------------
FROM rust:1.40 AS builder
WORKDIR /usr/src/running-mate
# Install Musl support
RUN rustup target add x86_64-unknown-linux-musl
# Copy over the code & build it
COPY . .
RUN cargo build --target x86_64-unknown-linux-musl --release

# Final image -----------------------------------------------------------------
FROM alpine:3
COPY --from=builder \
  /usr/src/running-mate/target/x86_64-unknown-linux-musl/release/running-mate \
  /usr/local/bin/running-mate
CMD ["running-mate"]
