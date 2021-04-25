FROM rust:1-slim-buster AS base

ENV USER=root

WORKDIR /code
RUN cargo init
COPY leo_shared/Cargo.toml /code/leo_shared/Cargo.toml
COPY leo_shared/src /code/leo_shared/src
COPY leo_bot/Cargo.toml /code/leo_bot/Cargo.toml
COPY leo_bot/src /code/leo_bot/src

WORKDIR /code/leo_bot

RUN cargo fetch

CMD [ "cargo", "test", "--offline" ]

FROM base AS builder

RUN cargo build --release --offline

FROM rust:1-slim-buster

COPY --from=builder /code/leo_bot/target/release/leo_bot /usr/bin/leo_bot

WORKDIR /usr/bin/web

ENTRYPOINT [ "/usr/bin/leo_bot" ]