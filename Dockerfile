# syntax=docker/dockerfile:1.4 # Specifies the Dockerfile syntax version

# Stage 1: Build the backend-project
# Replace 'rust:latest' with the appropriate base image for your build environment (e.g., golang:latest, maven:latest, node:latest)
FROM rust:1.77.2-slim-bookworm AS builder

WORKDIR /app

# Copy your project's files into the builder stage
# Adjust the COPY command based on your project structure
COPY . .

# Build your backend-project
# IMPORTANT: Ensure your binary is statically linked for the final 'scratch' image.
# For Rust: Use a musl target for static linking.
# For Go: Use CGO_ENABLED=0 and -ldflags "-s -w"
# Replace with your actual build command
RUN apt-get update && apt-get install -y musl-tools \
    && rustup target add x86_64-unknown-linux-musl \
    && cargo build --release --target x86_64-unknown-linux-musl

# Stage 2: Create the final minimal image
FROM scratch

# Copy the statically linked binary from the builder stage
# Adjust the path to your compiled binary
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/backend-project /backend-project

# Make the binary executable
RUN chmod +x /backend-project

# Expose the port your application listens on
EXPOSE 8080

# Define the command to run your application
CMD ["/backend-project"]