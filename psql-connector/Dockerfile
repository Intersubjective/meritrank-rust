FROM ghcr.io/intersubjective/pgrx-toolchain:pg17-pgrx0.17.0 AS connector

WORKDIR /usr/project

# --- Dependency cache layer (rebuilt only when Cargo.toml/lock change) ---
COPY core/Cargo.toml core/Cargo.toml
COPY service/Cargo.toml service/Cargo.toml
COPY psql-connector/Cargo.toml psql-connector/Cargo.toml
COPY psql-connector/Cargo.lock psql-connector/Cargo.lock

RUN mkdir -p core/src service/src psql-connector/src && \
    echo "" > core/src/lib.rs && \
    echo "" > service/src/lib.rs && \
    echo "fn main() {}" > service/src/main.rs && \
    echo "" > psql-connector/src/lib.rs && \
    echo "::pgrx::pgrx_embed!();" > psql-connector/src/pgrx_embed.rs

WORKDIR /usr/project/psql-connector
RUN cargo build --lib --release --features pg17 --no-default-features && \
    rm -rf /usr/project/target/release/.fingerprint/pgmer2-* \
           /usr/project/target/release/.fingerprint/meritrank_core-* \
           /usr/project/target/release/.fingerprint/meritrank_service-* \
           /usr/project/target/release/deps/libpgmer2* \
           /usr/project/target/release/deps/libmeritrank*

# --- Real source layer ---
WORKDIR /usr/project
COPY . .

WORKDIR /usr/project/psql-connector
RUN cargo pgrx package
RUN sh generate_scripts.sh target/release/pgmer2-pg17/usr/local/share/postgresql/extension

FROM postgres:17-alpine

ENV MERITRANK_SERVICE_URL=tcp://127.0.0.1:10234
ENV MERITRANK_RECV_TIMEOUT_MSEC=10000

COPY --from=connector /usr/project/psql-connector/target/release/pgmer2-pg17/usr/local/lib/postgresql/pgmer2.so /usr/local/lib/postgresql/pgmer2.so
COPY --from=connector /usr/project/psql-connector/extension /usr/local/share/postgresql/extension
COPY psql-connector/20_pgmer2.sh /docker-entrypoint-initdb.d/20_pgmer2.sh
