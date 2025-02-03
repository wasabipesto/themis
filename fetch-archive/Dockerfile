FROM rust:1.75 as builder
WORKDIR /usr/src/themis-fetch

COPY . .
#RUN cargo test
RUN cargo build -r

FROM debian:bookworm-slim
WORKDIR /usr/src/themis-fetch

RUN apt-get update
RUN apt-get install ca-certificates libpq5 -y
COPY --from=builder /usr/src/themis-fetch/target/release/themis-fetch .

CMD ["./themis-fetch"]