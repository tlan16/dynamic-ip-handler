FROM rust:1-slim-bookworm AS builder

WORKDIR /app
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
WORKDIR /app
ENV PATH="$PATH:/app"
COPY --from=builder /app/target/release/dynamic-ip-handler /app/dynamic-ip-handler
CMD ["dynamic-ip-handler"]
