FROM rust:1.75.0-alpine as compile
RUN apk add cmake make gcc musl-dev

WORKDIR /usr/project
COPY . .
WORKDIR /usr/project/service
RUN MERITRANK_NO_ASSERT=1 cargo build --release

FROM scratch
EXPOSE 10234

ENV MERITRANK_SERVICE_URL=tcp://127.0.0.1:10234
ENV MERITRANK_SERVICE_THREADS=32
ENV MERITRANK_NUM_WALK=10000
ENV MERITRANK_ZERO_NODE=U000000000000
ENV MERITRANK_TOP_NODES_LIMIT=100
ENV MERITRANK_FILTER_NUM_HASHES=10
ENV MERITRANK_FILTER_MIN_SIZE=32
ENV MERITRANK_FILTER_MAX_SIZE=8192

WORKDIR /srv
ENTRYPOINT [ "/srv/meritrank_service" ]
COPY --from=compile /usr/project/service/target/release/meritrank_service meritrank_service
