FROM rust:1.91.1 as builder
WORKDIR /app
COPY . .
RUN apt-get update && apt-get install -y musl-tools pkg-config perl make
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM gcr.io/distroless/static-debian12
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/kastlewatch /
CMD ["/kastlewatch"]
