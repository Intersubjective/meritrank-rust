#!/usr/bin/env bash
# Run test_psql_connector the same way as CI: same image, same steps, service started
# in a separate "step" so it reparents to pid 1 and survives into the test step.
# Usage: from repo root, run: ./psql-connector/run_tests_like_ci.sh
# Requires: Docker, and optionally docker login ghcr.io for the pgrx-toolchain image.

set -e
IMAGE="${PGRX_IMAGE:-ghcr.io/intersubjective/pgrx-toolchain:pg17-pgrx0.17.0}"
CONTAINER_NAME="meritrank-pgrx-test-$$"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "== Using image: $IMAGE"
echo "== Repo root: $REPO_ROOT"

docker pull "$IMAGE" || true
docker run -d --name "$CONTAINER_NAME" \
  -e PGRX_HOME=/root/.pgrx \
  -e RUST_BACKTRACE=1 \
  -v "$REPO_ROOT":/repo \
  -w /repo \
  "$IMAGE" \
  tail -f /dev/null

cleanup() {
  echo "== Stopping and removing container $CONTAINER_NAME"
  docker rm -f "$CONTAINER_NAME" 2>/dev/null || true
}
trap cleanup EXIT

run() {
  docker exec -e PGRX_HOME=/root/.pgrx -e RUST_BACKTRACE=1 "$CONTAINER_NAME" sh -c "$*"
}

echo "== Clean host-built artifacts (avoid rustc version mismatch)"
run "cd /repo/psql-connector && cargo clean"
run "cd /repo/service && cargo clean"

echo "== Build the connector"
run "cd /repo/psql-connector && cargo build"

echo "== Build the service (release)"
run "cd /repo/service && unset CARGO_TARGET_DIR && cargo build --release"

echo "== Install sudo and netcat-openbsd"
run "apk add --no-cache sudo netcat-openbsd"

echo "== Configure passwordless sudo (avoid pgrx 'sudo cp' hanging in non-interactive run)"
run "mkdir -p /etc/sudoers.d && echo 'Defaults !requiretty' > /etc/sudoers.d/nopasswd && echo 'Defaults env_keep += \"MERITRANK_SERVICE_URL\"' >> /etc/sudoers.d/nopasswd && echo 'root ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers.d/nopasswd && chmod 440 /etc/sudoers.d/nopasswd"

echo "== Start meritrank service (step exits so process reparents to pid 1)"
run "cd /repo && export MERITRANK_SERVER_PORT=10234 MERITRANK_SERVER_ADDRESS=127.0.0.1 MERITRANK_NUM_WALKS=500 && nohup ./target/release/meritrank_service >> /tmp/meritrank.log 2>&1 & i=0; while [ \$i -lt 30 ]; do nc -z 127.0.0.1 10234 && break; i=\$((i+1)); sleep 1; done; nc -z 127.0.0.1 10234 || (echo 'Meritrank service did not start'; cat /tmp/meritrank.log 2>/dev/null; exit 1)"

echo "== Do the tests"
# Clean pgrx test data so initdb doesn't fail with "directory exists but is not empty" (CI has a fresh workspace; local runs reuse the mount)
run "cd /repo/psql-connector && rm -rf ../target/test-pgdata && export RUST_TEST_THREADS=1 MERITRANK_SERVICE_URL=tcp://127.0.0.1:10234 && mkdir -p /usr/local/share/postgresql/extension /usr/local/lib/postgresql && chmod -R a+rwx /usr/local/share/postgresql/extension /usr/local/lib/postgresql 2>/dev/null || true && mkdir -p ../target/test-pgdata && chown -R postgres:postgres ../target/test-pgdata && unset PG_VERSION && nc -z 127.0.0.1 10234 || (echo 'Meritrank service died before tests'; cat /tmp/meritrank.log 2>/dev/null; exit 1) && cargo pgrx test --runas postgres"
r=$?

echo "--- meritrank service log ---"
run "cat /tmp/meritrank.log 2>/dev/null || true"

exit $r
