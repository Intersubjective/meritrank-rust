#!/bin/bash

set -e

export PGUSER="$POSTGRES_USER"

echo "Loading pgmer2 extension into $POSTGRES_DB"
"${psql[@]}" --dbname="$POSTGRES_DB" <<-'EOSQL'
  CREATE EXTENSION IF NOT EXISTS pgmer2;
EOSQL
