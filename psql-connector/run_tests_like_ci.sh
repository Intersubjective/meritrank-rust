#!/usr/bin/env bash
# Run meritrank service and pgrx tests the same way CI does (see .github/workflows/build_and_test.yml / publish.yml)
set -e
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

# Build connector first (CI: cargo build in psql-connector)
cd psql-connector
cargo build --quiet 2>/dev/null || true
cd "$REPO_ROOT"

# Build and run the service in background (CI: Build and run the service)
cd service
cargo build --release --quiet 2>/dev/null || true
export MERITRANK_SERVER_PORT=10234
export MERITRANK_SERVER_ADDRESS=127.0.0.1
export MERITRANK_NUM_WALKS=500
cargo run --release --bin meritrank_service 2>&1 &
SERVICE_PID=$!
cd "$REPO_ROOT"

# Wait for service to bind
echo "Waiting for meritrank service to listen on 10234..."
for i in $(seq 1 15); do
  if ss -ltn 2>/dev/null | grep -q 10234 || netstat -ltn 2>/dev/null | grep -q 10234; then
    echo "Service is up."
    break
  fi
  sleep 1
done
if ! (ss -ltn 2>/dev/null | grep -q 10234) && ! (netstat -ltn 2>/dev/null | grep -q 10234); then
  echo "Warning: service may not be listening on 10234 yet"
fi

# Do the tests (CI: RUST_TEST_THREADS=1, cargo pgrx test --runas postgres --pgdata /var/lib/postgresql/pgrx)
# Omit --runas/--pgdata if not available locally so tests still run
cd psql-connector
export MERITRANK_SERVICE_URL=tcp://127.0.0.1:10234
export RUST_TEST_THREADS=1
if id postgres &>/dev/null && [ -d /var/lib/postgresql/pgrx ] 2>/dev/null; then
  cargo pgrx test pg17 --runas postgres --pgdata /var/lib/postgresql/pgrx
else
  cargo pgrx test pg17
fi
RESULT=$?

# Kill service
kill $SERVICE_PID 2>/dev/null || true
exit $RESULT
