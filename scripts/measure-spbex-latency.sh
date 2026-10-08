#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
run_dir="$(mktemp -d)"
listen_addr="127.0.0.1:18080"
server_pid=""
cleanup() {
  if [[ -n "$server_pid" ]]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -rf "$run_dir"
}
trap cleanup EXIT

cd "$repo_root"
cargo build --release --locked
LISTEN_ADDR="$listen_addr" \
MOEX_ISS_BASE_URL="https://iss.moex.com/iss/" \
SPBEX_API_BASE_URL="https://spbexchange.ru/api/" \
MOEX_HISTORY_CACHE_DB_PATH="$run_dir/history.sqlite3" \
./target/release/exchange-api >"$run_dir/server.log" 2>&1 &
server_pid=$!

for _ in $(seq 1 100); do
  if curl --silent --fail http://127.0.0.1:18080/health/ready >/dev/null; then
    break
  fi
  sleep 0.1
done
curl --silent --fail http://127.0.0.1:18080/health/ready >/dev/null || {
  cat "$run_dir/server.log" >&2
  exit 1
}

python3 scripts/measure-spbex-latency.py "$@"
