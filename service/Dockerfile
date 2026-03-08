FROM rust:alpine AS compile
RUN apk add cmake make gcc musl-dev

WORKDIR /usr/project

# --- Dependency cache layer ---
COPY Cargo.toml Cargo.lock ./
COPY core/Cargo.toml core/Cargo.toml
COPY service/Cargo.toml service/Cargo.toml
COPY psql-connector/Cargo.toml psql-connector/Cargo.toml

RUN mkdir -p core/src service/src psql-connector/src && \
    echo "" > core/src/lib.rs && \
    echo "" > service/src/lib.rs && \
    echo "fn main() {}" > service/src/main.rs && \
    echo "" > psql-connector/src/lib.rs && \
    echo "fn main() {}" > psql-connector/src/pgrx_embed.rs

RUN cargo build --release -p meritrank_service && \
    rm -rf /usr/project/target/release/.fingerprint/meritrank_core-* \
           /usr/project/target/release/.fingerprint/meritrank_service-* \
           /usr/project/target/release/deps/libmeritrank* \
           /usr/project/target/release/meritrank_service

# --- Real source layer ---
COPY . .

RUN cargo build --release -p meritrank_service

FROM scratch
EXPOSE 10234

ENV MERITRANK_SERVER_PORT=10234
ENV MERITRANK_SERVER_ADDRESS=127.0.0.1
ENV MERITRANK_NUM_WALKS=10000
ENV MERITRANK_ZERO_OPINION_NUM_WALKS=1000
ENV MERITRANK_TOP_NODES_LIMIT=100
ENV MERITRANK_ZERO_OPINION_FACTOR=0.2
ENV MERITRANK_SCORE_CLUSTERS_CACHE_SIZE=10240
ENV MERITRANK_SCORE_CLUSTERS_TIMEOUT=21600
ENV MERITRANK_SCORES_CACHE_SIZE=10240
ENV MERITRANK_SCORES_CACHE_TIMEOUT=3600
ENV MERITRANK_OMIT_NEG_EDGES_SCORES=false
ENV MERITRANK_FORCE_READ_GRAPH_CONN=false
ENV MERITRANK_NUM_SCORE_QUANTILES=100
ENV MERITRANK_SLEEP_DURATION_AFTER_PUBLISH_MS=10
ENV MERITRANK_SUBGRAPH_QUEUE_CAPACITY=1024

WORKDIR /srv
ENTRYPOINT [ "/srv/meritrank_service" ]
COPY --from=compile /usr/project/target/release/meritrank_service meritrank_service
