"""Production-source history cache lifecycle probe shared by provider harnesses."""

import http.client
import re
import time
import urllib.parse


def request_once(base_url, provider, symbol):
    endpoint = urllib.parse.urlsplit(base_url)
    path = f"/v1/{provider.lower()}/{symbol}"
    connection = http.client.HTTPConnection(endpoint.hostname, endpoint.port, timeout=40)
    started = time.perf_counter()
    connection.request("GET", path)
    response = connection.getresponse()
    response.read()
    connection.close()
    return response.status, time.perf_counter() - started


def read_events(log_path, offset, provider):
    with open(log_path, "rb") as log:
        log.seek(offset)
        text = log.read().decode("utf-8", errors="replace")
        next_offset = log.tell()
    source_fetches = []
    outcomes = []
    for line in text.splitlines():
        if f'provider="{provider.upper()}"' not in line:
            continue
        symbol = re.search(r'symbol="?([A-Z0-9._-]+)"?', line)
        if not symbol:
            continue
        if 'phase="source_fetch"' in line:
            source_fetches.append(symbol.group(1))
        elif "history request completed" in line:
            cache = re.search(r'cache="(memory_hit|persistent_hit|miss)"', line)
            if cache:
                outcomes.append((symbol.group(1), cache.group(1)))
    return next_offset, source_fetches, outcomes


def run_lifecycle(args, provider):
    if not args.server_log:
        raise SystemExit("--server-log is required for the history cache lifecycle profile")
    if args.history_cache_ttl_seconds <= 0:
        raise SystemExit("history-cache TTL must be positive")
    symbol = args.symbols[0]
    offset = 0

    cold_status, cold_seconds = request_once(args.base_url, provider, symbol)
    offset, cold_fetches, cold_outcomes = read_events(args.server_log, offset, provider)
    warm_status, warm_seconds = request_once(args.base_url, provider, symbol)
    offset, warm_fetches, warm_outcomes = read_events(args.server_log, offset, provider)
    time.sleep(args.history_cache_ttl_seconds + 1)
    expiry_status, expiry_seconds = request_once(args.base_url, provider, symbol)
    _, expiry_fetches, expiry_outcomes = read_events(args.server_log, offset, provider)

    cold_ok = cold_status == 200 and cold_fetches == [symbol] and (symbol, "miss") in cold_outcomes
    warm_ok = warm_status == 200 and not warm_fetches and (symbol, "memory_hit") in warm_outcomes
    expiry_ok = expiry_status == 200 and expiry_fetches == [symbol] and (symbol, "miss") in expiry_outcomes
    print(
        f"history_cache_lifecycle provider={provider.upper()} symbol={symbol} "
        f"cold_status={cold_status} cold_seconds={cold_seconds:.6f} "
        f"cold_source_fetches={len(cold_fetches)} cold_cache={cold_outcomes} "
        f"warm_status={warm_status} warm_seconds={warm_seconds:.6f} "
        f"warm_source_fetches={len(warm_fetches)} warm_cache={warm_outcomes} "
        f"expiry_status={expiry_status} expiry_seconds={expiry_seconds:.6f} "
        f"expiry_source_fetches={len(expiry_fetches)} expiry_cache={expiry_outcomes} "
        f"ttl_seconds={args.history_cache_ttl_seconds} "
        f"verified={str(cold_ok and warm_ok and expiry_ok).lower()}",
        flush=True,
    )
    return cold_ok and warm_ok and expiry_ok
