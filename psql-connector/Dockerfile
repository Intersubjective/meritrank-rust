FROM postgres:16-alpine AS pgsodium

RUN apk add make clang19 musl-dev postgresql16-dev curl git
WORKDIR /usr/project

RUN curl -s -L https://download.libsodium.org/libsodium/releases/libsodium-1.0.18.tar.gz | tar zxvf - && cd libsodium-1.0.18 && ./configure && make check && make -j 4 install
RUN git clone --depth 1 --branch v3.1.9 https://github.com/michelp/pgsodium.git && cd pgsodium && make -j 4

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

COPY --from=pgsodium /usr/local/lib/libsodium.so.23.3.0 /usr/local/lib/libsodium.so.23.3.0
RUN cd /usr/local/lib && ln -s -f libsodium.so.23.3.0 libsodium.so.23
COPY --from=pgsodium /usr/project/pgsodium/pgsodium.so /usr/local/lib/postgresql/pgsodium.so
COPY --from=pgsodium /usr/project/pgsodium/pgsodium.control /usr/local/share/postgresql/extension/pgsodium.control
COPY --from=pgsodium /usr/project/pgsodium/sql /usr/local/share/postgresql/extension

COPY --from=connector /usr/project/psql-connector/target/release/pgmer2-pg16/usr/local/lib/postgresql/pgmer2.so /usr/local/lib/postgresql/pgmer2.so
COPY --from=connector /usr/project/psql-connector/extension /usr/local/share/postgresql/extension
COPY psql-connector/20_pgmer2.sh /docker-entrypoint-initdb.d/20_pgmer2.sh
