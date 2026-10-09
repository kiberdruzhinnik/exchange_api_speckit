"""Verify the persistent incremental history lifecycle against the running service."""

import http.client
import re
import urllib.parse


def request_once(base_url, provider, symbol):
    endpoint = urllib.parse.urlsplit(base_url)
    path = f"/v1/{provider.lower()}/{symbol}"
    connection = http.client.HTTPConnection(endpoint.hostname, endpoint.port, timeout=40)
    connection.request("GET", path)
    response = connection.getresponse()
    body = response.read()
    connection.close()
    return response.status, body


def read_events(log_path, offset, provider):
    with open(log_path, "rb") as log:
        log.seek(offset)
        text = log.read().decode("utf-8", errors="replace")
        next_offset = log.tell()
    fetches = []
    outcomes = []
    for line in text.splitlines():
        if f'provider="{provider.upper()}"' not in line:
            continue
        symbol = re.search(r'symbol="?([A-Z0-9._-]+)"?', line)
        if not symbol:
            continue
        if 'phase="source_fetch"' in line:
            fetches.append(symbol.group(1))
        elif "history request completed" in line:
            cache = re.search(r'cache="([a-z_]+)"', line)
            if cache:
                outcomes.append((symbol.group(1), cache.group(1)))
    return next_offset, fetches, outcomes


def run_lifecycle(args, provider):
    if not args.server_log:
        raise SystemExit("--server-log is required for the incremental history lifecycle profile")
    symbol = args.symbols[0]
    offset = 0
    results = []
    for _ in range(3):
        status, body = request_once(args.base_url, provider, symbol)
        offset, fetches, outcomes = read_events(args.server_log, offset, provider)
        results.append((status, body, fetches, outcomes))
    successful = all(status == 200 for status, _, _, _ in results)
    fetches_ok = all(fetches == [symbol] for _, _, fetches, _ in results)
    json_ok = all(body.startswith(b"[") for _, body, _, _ in results)
    verified = successful and fetches_ok and json_ok
    print(
        f"history_incremental_lifecycle provider={provider.upper()} symbol={symbol} "
        f"statuses={[status for status, _, _, _ in results]} "
        f"source_fetches={[len(fetches) for _, _, fetches, _ in results]} "
        f"refresh_outcomes={[outcomes for _, _, _, outcomes in results]} "
        f"verified={str(verified).lower()}",
        flush=True,
    )
    return verified
