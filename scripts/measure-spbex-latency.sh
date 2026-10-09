#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
run_dir="$(mktemp -d)"
listen_addr="127.0.0.1:18080"
server_pid=""
benchmark_args=("$@")
benchmark_profile="all"
benchmark_symbols="SBER"
for ((index = 0; index < ${#benchmark_args[@]}; index++)); do
  case "${benchmark_args[index]}" in
    --profile) benchmark_profile="${benchmark_args[index + 1]}" ;;
    --profile=*) benchmark_profile="${benchmark_args[index]#*=}" ;;
    --symbols) benchmark_symbols="${benchmark_args[index + 1]}" ;;
    --symbols=*) benchmark_symbols="${benchmark_args[index]#*=}" ;;
  esac
done
benchmark_ttl=300
if [[ "$benchmark_profile" == "lifecycle" ]]; then benchmark_ttl=60; fi
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
EXCHANGE_API_LISTEN_ADDR="$listen_addr" \
EXCHANGE_API_HISTORY_CACHE_TTL_SECS="$benchmark_ttl" \
EXCHANGE_API_MOEX_ISS_BASE_URL="https://iss.moex.com/iss/" \
EXCHANGE_API_SPBEX_API_BASE_URL="https://spbexchange.ru/api/" \
EXCHANGE_API_HISTORY_CACHE_DB_PATH="$run_dir/history.sqlite3" \
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
if [[ "$benchmark_profile" != "lifecycle" ]]; then
  IFS=',' read -r -a warm_symbols <<< "$benchmark_symbols"
  for symbol in "${warm_symbols[@]}"; do
    curl --silent --fail "http://127.0.0.1:18080/v1/spbex/$symbol" >/dev/null
  done
fi

python3 scripts/measure-spbex-latency.py --server-log "$run_dir/server.log" --history-cache-ttl-seconds "$benchmark_ttl" "$@"
