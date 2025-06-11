# syntax=docker/dockerfile:1.4

# Stage 1: Build the backend-project
# Use the official Rust nightly-slim image
FROM rustlang/rust:nightly-slim AS builder

WORKDIR /app

# Copy your project's files into the builder stage
COPY . .

# Build your backend-project
# Ensure musl-tools are installed for static linking
RUN apt-get update && apt-get install -y musl-tools \
    && rustup target add x86_64-unknown-linux-musl \
    && SQLX_OFFLINE=true cargo build --release --target x86_64-unknown-linux-musl

# Stage 2: Create the final minimal image
FROM scratch

# Copy the statically linked binary from the builder stage
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/backend-project /

# Expose the port your application listens on
EXPOSE 8080

# Define the command to run your application
CMD ["./backend-project"]