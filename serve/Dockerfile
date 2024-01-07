FROM rust:1.75 as builder
WORKDIR /usr/src/themis-serve

COPY . .
#RUN cargo test
RUN cargo build -r

FROM debian:bookworm-slim
WORKDIR /usr/src/themis-serve

RUN apt-get update
RUN apt-get install libpq5 -y
COPY --from=builder /usr/src/themis-serve/target/release/themis-serve .

CMD ["./themis-serve"]