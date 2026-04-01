# Build stage
FROM rust:1.75-bookworm AS builder
RUN apt-get update && apt-get install -y protobuf-compiler
WORKDIR /app
COPY . .
RUN cargo build --workspace --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/nstn-server /usr/local/bin/
COPY --from=builder /app/config /etc/nanosistant/config
COPY --from=builder /app/proto /etc/nanosistant/proto
EXPOSE 3000 50051
ENV NSTN_CONFIG_DIR=/etc/nanosistant/config
ENV NSTN_DATA_DIR=/var/lib/nanosistant
ENV RUST_LOG=info
CMD ["nstn-server"]
