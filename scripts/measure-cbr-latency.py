#!/usr/bin/env python3
"""Measure complete CBR API responses under a fixed 10 requests/s arrival rate."""

import argparse
import concurrent.futures
import http.client
import math
import re
import time
import urllib.parse


def request_once(url, route, symbol, client_id):
    started = time.perf_counter()
    issued_at = time.monotonic()
    try:
        endpoint = urllib.parse.urlsplit(url)
        connection = http.client.HTTPConnection(endpoint.hostname, endpoint.port, timeout=40)
        path = endpoint.path + (f"?{endpoint.query}" if endpoint.query else "")
        connection.request("GET", path, headers={"X-Benchmark-Client": str(client_id)})
        response = connection.getresponse()
        response.read()
        connection.close()
        error = None if 200 <= response.status < 300 else f"HTTP {response.status}"
        return route, symbol, time.perf_counter() - started, issued_at, response.status, error
    except Exception as error:  # Report transport failures separately from successful latency.
        return route, symbol, time.perf_counter() - started, issued_at, None, str(error)


def read_cache_events(log_path, offset):
    if not log_path:
        return offset, [], []
    with open(log_path, "rb") as log:
        log.seek(offset)
        contents = log.read()
        new_offset = log.tell()
    text = contents.decode("utf-8", errors="replace")
    source_fetches = []
    outcomes = []
    for line in text.splitlines():
        if 'provider="CBR"' not in line:
            continue
        symbol = re.search(r'symbol="?([A-Z]{3})"?', line)
        if not symbol:
            continue
        if 'phase="source_fetch"' in line:
            source_fetches.append(symbol.group(1))
        elif 'history request completed' in line:
            cache = re.search(r'cache="(memory_hit|persistent_hit|miss)"', line)
            if cache:
                outcomes.append((symbol.group(1), cache.group(1)))
    return new_offset, source_fetches, outcomes


def issue_history_probe(args, symbol=None):
    symbol = symbol or args.symbols[0]
    url = f"{args.base_url.rstrip('/')}/v1/cbr/{symbol}"
    result = request_once(url, "history", symbol, 0)
    args.log_offset, source_fetches, outcomes = read_cache_events(
        args.server_log, args.log_offset
    )
    return result, source_fetches, outcomes


def verify_history_cache_lifecycle(args):
    cold, cold_fetches, cold_outcomes = issue_history_probe(args)
    warm, warm_fetches, warm_outcomes = issue_history_probe(args)
    time.sleep(args.history_cache_ttl_seconds + 1)
    expired, expiry_fetches, expiry_outcomes = issue_history_probe(args)
    source_fetch_count = len(cold_fetches) + len(warm_fetches) + len(expiry_fetches)
    warm_hit_count = sum(
        outcome == "memory_hit"
        for _, outcome in cold_outcomes + warm_outcomes + expiry_outcomes
    )
    statuses_ok = all(row[4] is not None and 200 <= row[4] < 300 for row in (cold, warm, expired))
    verified = statuses_ok and len(cold_fetches) == 1 and not warm_fetches and len(expiry_fetches) == 1 and warm_hit_count >= 1
    print(
        f"history_cache_lifecycle cold_status={cold[4]} cold_source_fetches={len(cold_fetches)} "
        f"warm_status={warm[4]} warm_memory_hits={warm_hit_count} "
        f"expiry_status={expired[4]} expiry_source_fetches={len(expiry_fetches)} "
        f"ttl_seconds={args.history_cache_ttl_seconds} verified={str(verified).lower()}",
        flush=True,
    )
    return verified


def run_stage(args, mode):
    warmup_statuses = []
    if mode in ("history", "combined"):
        warmup_statuses = [
            issue_history_probe(args, symbol)[0][4] for symbol in args.symbols
        ]
    history_warmup_ok = all(
        status is not None and 200 <= status < 300 for status in warmup_statuses
    ) if warmup_statuses else True
    started = time.monotonic()
    pending = {}
    active_clients = set()
    results = []
    issued_count = 0
    skipped_count = 0
    next_client_id = 0
    schedule_slots = math.ceil(args.duration_seconds * 10)
    with concurrent.futures.ThreadPoolExecutor(max_workers=10) as pool:
        slot = 0
        while slot < schedule_slots:
            delay = started + slot / 10.0 - time.monotonic()
            if delay > 0:
                time.sleep(delay)
            due_slot = min(schedule_slots - 1, math.floor((time.monotonic() - started) * 10))
            if due_slot > slot:
                skipped_count += due_slot - slot
                slot = due_slot
            for future in [item for item in pending if item.done()]:
                results.append(future.result())
                active_clients.remove(pending.pop(future))
            available = [
                (next_client_id + offset) % 10
                for offset in range(10)
                if (next_client_id + offset) % 10 not in active_clients
            ]
            if not available:
                skipped_count += 1
                slot += 1
                continue
            client_id = available[0]
            next_client_id = (client_id + 1) % 10
            route = ("history" if slot % 2 == 0 else "quote") if mode == "combined" else mode
            symbol = args.symbols[slot % len(args.symbols)]
            suffix = "" if route == "history" else "/quote"
            url = f"{args.base_url.rstrip('/')}/v1/cbr/{symbol}{suffix}"
            future = pool.submit(request_once, url, route, symbol, client_id)
            pending[future] = client_id
            active_clients.add(client_id)
            issued_count += 1
            slot += 1
        for future in pending:
            results.append(future.result())

    args.log_offset, source_fetches, cache_outcomes = read_cache_events(
        args.server_log, args.log_offset
    )
    memory_hits = sum(outcome == "memory_hit" for _, outcome in cache_outcomes)
    persistent_hits = sum(outcome == "persistent_hit" for _, outcome in cache_outcomes)
    cache_verified = history_warmup_ok

    successes = [row for row in results if row[4] is not None and 200 <= row[4] < 300]
    errors = [row for row in results if row[4] is None or not 200 <= row[4] < 300]
    latencies = sorted(row[2] for row in successes)
    p95 = latencies[max(0, math.ceil(len(latencies) * 0.95) - 1)] if latencies else None
    actual_rate = issued_count / max(args.duration_seconds, 0.001)
    issue_times = sorted(row[3] for row in results)
    observed_rate = (
        (len(issue_times) - 1) / max(issue_times[-1] - issue_times[0], 0.001)
        if len(issue_times) > 1
        else 0.0
    )
    valid_arrivals = issued_count == schedule_slots and skipped_count == 0
    print(
        f"profile={mode} scheduled={schedule_slots} issued={issued_count} skipped={skipped_count} "
        f"successful={len(successes)} errors={len(errors)} clients=10 target_rps=10 "
        f"arrival_schedule_valid={str(valid_arrivals).lower()} "
        f"actual_issue_rps={actual_rate:.2f} observed_issue_rps={observed_rate:.2f} p95_seconds={p95:.6f}" if p95 is not None else
        f"profile={mode} scheduled={schedule_slots} issued={issued_count} skipped={skipped_count} "
        f"successful=0 errors={len(errors)} clients=10 target_rps=10 arrival_schedule_valid={str(valid_arrivals).lower()} "
        f"actual_issue_rps={actual_rate:.2f} observed_issue_rps={observed_rate:.2f} p95_seconds=unavailable",
        flush=True,
    )
    if mode in ("history", "combined"):
        print(
            f"profile={mode} history_source_fetches={len(source_fetches)} "
            f"history_warm_memory_hits={memory_hits} history_persistent_hits={persistent_hits} "
            f"history_warmup_statuses={','.join(str(status) for status in warmup_statuses)}",
            flush=True,
        )
    route_ok = True
    for route in (("history", "quote") if mode == "combined" else (mode,)):
        route_latencies = sorted(row[2] for row in successes if row[0] == route)
        if route_latencies:
            index = max(0, math.ceil(len(route_latencies) * 0.95) - 1)
            print(f"profile={mode} route={route} successful={len(route_latencies)} p95_seconds={route_latencies[index]:.6f}", flush=True)
            route_ok = route_ok and route_latencies[index] < 1.0
        else:
            route_ok = False
    for _, symbol, _, _, status, error in errors[:20]:
        print(f"error=symbol:{symbol} status:{status} detail:{error}", flush=True)
    return valid_arrivals and route_ok and cache_verified


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-url", default="http://127.0.0.1:18080")
    parser.add_argument("--duration-seconds", type=int, default=55)
    parser.add_argument("--symbols", default="USD")
    parser.add_argument("--server-log")
    parser.add_argument("--history-cache-ttl-seconds", type=int, default=60)
    parser.add_argument(
        "--profile", choices=("all", "history", "quote", "combined"), default="all"
    )
    args = parser.parse_args()
    endpoint = urllib.parse.urlsplit(args.base_url)
    if endpoint.scheme != "http" or endpoint.hostname not in {"127.0.0.1", "localhost", "::1"}:
        parser.error("base URL must target the local HTTP service")
    args.symbols = [symbol.strip().upper() for symbol in args.symbols.split(",") if symbol.strip()]
    if not args.symbols or args.duration_seconds <= 0 or args.history_cache_ttl_seconds <= 0:
        parser.error("duration and history-cache TTL must be positive and at least one symbol is required")
    if args.duration_seconds >= args.history_cache_ttl_seconds:
        parser.error("each timed profile must be shorter than the history-cache TTL; cache expiry is verified separately")
    print("profile=10_clients target_total_rps=10 body_fully_read=true", flush=True)
    args.log_offset = 0
    if not args.server_log:
        parser.error("--server-log is required to verify the history cache lifecycle")
    lifecycle_verified = verify_history_cache_lifecycle(args)
    modes = ("history", "quote", "combined") if args.profile == "all" else (args.profile,)
    profiles_passed = [run_stage(args, mode) for mode in modes]
    if not lifecycle_verified or not all(profiles_passed):
        raise SystemExit("invalid arrival profile or successful-response p95 acceptance failure")


if __name__ == "__main__":
    main()
