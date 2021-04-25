FROM rust:1-slim-buster AS base

ENV USER=root

WORKDIR /code
RUN cargo init
COPY leo_shared/Cargo.toml /code/leo_shared/Cargo.toml
COPY leo_shared/src /code/leo_shared/src
COPY leo_website/Cargo.toml /code/leo_website/Cargo.toml
COPY leo_website/src /code/leo_website/src
COPY templates /code/leo_website/templates

WORKDIR /code/leo_website

RUN cargo fetch

CMD [ "cargo", "test", "--offline" ]

FROM base AS builder

RUN cargo build --release --offline

FROM rust:1-slim-buster

COPY --from=builder /code/leo_website/target/release/leo_website /usr/bin/web/leo_website
COPY --from=builder /code/leo_website/templates /usr/bin/web/templates

WORKDIR /usr/bin/web

ENTRYPOINT [ "/usr/bin/web/leo_website" ]