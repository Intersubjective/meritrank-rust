FROM postgres:16-alpine AS pgsodium

RUN apk add make clang19 musl-dev postgresql16-dev curl git
WORKDIR /usr/project

RUN curl -s -L https://download.libsodium.org/libsodium/releases/libsodium-1.0.18.tar.gz | tar zxvf - && cd libsodium-1.0.18 && ./configure && make check && make -j 4 install
RUN git clone --depth 1 --branch v3.1.9 https://github.com/michelp/pgsodium.git && cd pgsodium && make -j 4

FROM postgres:16-alpine AS redis_fdw

#  TODO: Build

FROM postgres:16-alpine

COPY --from=pgsodium /usr/local/lib/libsodium.so.23.3.0 /usr/local/lib/libsodium.so.23.3.0
RUN cd /usr/local/lib && ln -s -f libsodium.so.23.3.0 libsodium.so.23
COPY --from=pgsodium /usr/project/pgsodium/pgsodium.so /usr/local/lib/postgresql/pgsodium.so
COPY --from=pgsodium /usr/project/pgsodium/pgsodium.control /usr/local/share/postgresql/extension/pgsodium.control
COPY --from=pgsodium /usr/project/pgsodium/sql /usr/local/share/postgresql/extension

#  TODO: Add Redis FDW

