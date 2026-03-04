# syntax=docker/dockerfile:1
FROM ghcr.io/intersubjective/pgrx-toolchain:pg17-pgrx0.17.0 AS connector

# Toolchain image no longer runs pgrx init; package needs PGRX_HOME.
RUN cargo pgrx init --pg17 pg_config

WORKDIR /usr/project
COPY . .

WORKDIR /usr/project/psql-connector
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/usr/project/target \
    cargo pgrx package && \
    cp -r target/release/pgmer2-pg17 /tmp/pgmer2-pg17

RUN sh generate_scripts.sh /tmp/pgmer2-pg17/usr/local/share/postgresql/extension

FROM postgres:17-alpine

ENV MERITRANK_SERVICE_URL=tcp://127.0.0.1:10234
ENV MERITRANK_RECV_TIMEOUT_MSEC=10000

COPY --from=connector /tmp/pgmer2-pg17/usr/local/lib/postgresql/pgmer2.so /usr/local/lib/postgresql/pgmer2.so
COPY --from=connector /usr/project/psql-connector/extension /usr/local/share/postgresql/extension
COPY psql-connector/20_pgmer2.sh /docker-entrypoint-initdb.d/20_pgmer2.sh
