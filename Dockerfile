#FROM rustlang/rust:nightly AS base
FROM clux/muslrust:nightly AS base

# https://jstoelm.com/episodes/40-rust-in-production-with-docker/
## TODO: https://blog.semicolonsoftware.de/building-minimal-docker-containers-for-rust-applications/

# Set the working directory to /app
WORKDIR /app

# Copy the current directory contents into the container at /app
COPY . .

# Build the app
RUN rustc --version && cargo build --release

# The command gets overridden by the docker-compose.yml so doesn't really matter that its wrong
CMD cargo run --release

## Move the binaries into a new container
FROM alpine:latest

COPY --from=base /app/target/x86_64-unknown-linux-musl/release/w3g-*-ms /app/target/x86_64-unknown-linux-musl/release/

COPY --from=base /etc/ssl/certs /etc/ssl/certs
ENV SSL_CERT_FILE="/etc/ssl/certs/ca-certificates.crt"
ENV SSL_CERT_DIR="/etc/ssl/certs"

CMD echo overriden