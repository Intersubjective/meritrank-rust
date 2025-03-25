FROM postgres:16-alpine AS connector

RUN apk add rust cargo rustfmt jq cmake make gcc llvm15 clang15 musl-dev openssl-dev postgresql16-dev jq
WORKDIR /usr/project
COPY . .

WORKDIR /usr/project/psql-connector
ENV RUSTFLAGS="-Ctarget-feature=-crt-static"
RUN cargo install --locked --version 0.12.8 cargo-pgrx
RUN cargo pgrx init --pg16 pg_config
RUN cargo pgrx package
RUN sh generate_scripts.sh target/release/pgmer2-pg16/usr/local/share/postgresql/extension

FROM postgres:16-alpine

ENV MERITRANK_SERVICE_URL=tcp://127.0.0.1:10234
ENV MERITRANK_RECV_TIMEOUT_MSEC=10000

COPY --from=connector /usr/project/psql-connector/target/release/pgmer2-pg16/usr/local/lib/postgresql/pgmer2.so /usr/local/lib/postgresql/pgmer2.so
COPY --from=connector /usr/project/psql-connector/extension /usr/local/share/postgresql/extension
COPY psql-connector/20_pgmer2.sh /docker-entrypoint-initdb.d/20_pgmer2.sh
