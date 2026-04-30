FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY Cargo.toml ./
COPY src ./src

RUN cargo build --release

FROM alpine:3.19

RUN apk add --no-cache ledger

COPY --from=builder /app/target/release/pennywise-backend /usr/local/bin/

EXPOSE 8080

CMD ["pennywise-backend"]
