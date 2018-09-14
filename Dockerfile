FROM rustlang/rust:nightly
# https://jstoelm.com/episodes/40-rust-in-production-with-docker/
## TODO: https://blog.semicolonsoftware.de/building-minimal-docker-containers-for-rust-applications/

# Set the working directory to /app
WORKDIR /app

# Copy the current directory contents into the container at /app
COPY . .

# Build the app before finishing the Dockerfile so that it doesn't happen when the container starts
RUN rustc --version && cargo build --release

# The command gets overridden by the docker-compose.yml so doesn't really matter that its wrong
CMD cargo run --release