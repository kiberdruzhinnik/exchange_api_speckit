#!/usr/bin/env python3
"""Measure complete local API response latency with a paced client workload."""

import argparse
import concurrent.futures
import http.client
import math
import threading
import time
import urllib.parse

from latency_lifecycle import run_lifecycle


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


def run_stage(args, mode):
    client_count = 10
    schedule_slots = args.duration_seconds * client_count
    start = time.monotonic() + 1.0
    active = 0
    max_in_flight = 0
    active_lock = threading.Lock()

    def issue(url, route, symbol, client_id):
        nonlocal active, max_in_flight
        with active_lock:
            active += 1
            max_in_flight = max(max_in_flight, active)
        try:
            return request_once(url, route, symbol, client_id)
        finally:
            with active_lock:
                active -= 1

    with concurrent.futures.ThreadPoolExecutor(max_workers=client_count) as pool:
        requests = []
        for slot in range(schedule_slots):
            scheduled_at = start + slot / 10
            delay = scheduled_at - time.monotonic()
            if delay > 0:
                time.sleep(delay)
            if mode == "combined":
                route = "history" if slot % 2 == 0 else "quote"
            else:
                route = mode
            symbol = args.symbols[slot % len(args.symbols)]
            suffix = "" if route == "history" else "/quote"
            url = f"{args.base_url.rstrip('/')}/v1/spbex/{symbol}{suffix}"
            requests.append(pool.submit(issue, url, route, symbol, slot % client_count))
        results = [request.result() for request in requests]
    skipped_count = 0
    issued_count = len(results)
    samples = [(route, latency) for route, _, latency, _, status, _ in results if status and 200 <= status < 300]
    errors = [
        {"symbol": symbol, "route": route, "status": status, "error": error}
        for route, symbol, _, _, status, error in results
        if status is None or not 200 <= status < 300
    ]
    issued_times = sorted(issued_at for _, _, _, issued_at, _, _ in results)
    latencies = sorted(value for _, value in samples)
    p95 = latencies[max(0, math.ceil(len(latencies) * 0.95) - 1)] if latencies else None
    elapsed = max(time.monotonic() - start, 0.001)
    issued_rps = issued_count / args.duration_seconds
    observed_start_rps = (
        len(issued_times) / max(issued_times[-1] - issued_times[0], 0.001)
        if issued_times
        else 0.0
    )
    completed_rps = len(results) / elapsed
    print(
        f"stage={mode} scheduled={schedule_slots} issued={issued_count} skipped={skipped_count} "
        f"successful={len(samples)} errors={len(errors)} max_in_flight={max_in_flight} scheduled_rps=10 "
        f"issued_rps={issued_rps:.2f} observed_start_rps={observed_start_rps:.2f} "
        f"completed_rps={completed_rps:.2f} "
        f"p95_seconds={p95:.6f}" if p95 is not None else
        f"stage={mode} scheduled={schedule_slots} issued={issued_count} skipped={skipped_count} "
        f"successful=0 errors={len(errors)} max_in_flight={max_in_flight} scheduled_rps=10 issued_rps={issued_rps:.2f} "
        f"observed_start_rps={observed_start_rps:.2f} completed_rps={completed_rps:.2f} "
        f"p95_seconds=unavailable",
        flush=True,
    )
    for route in ("history", "quote") if mode == "combined" else (mode,):
        route_samples = sorted(value for sample_route, value in samples if sample_route == route)
        if route_samples:
            route_p95 = route_samples[max(0, math.ceil(len(route_samples) * 0.95) - 1)]
            print(f"stage={mode} route={route} successful={len(route_samples)} p95_seconds={route_p95:.6f}", flush=True)
    for failure in errors[:20]:
        print(f"error={failure}", flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-url", default="http://127.0.0.1:18080")
    parser.add_argument("--duration-seconds", type=int, default=120)
    parser.add_argument("--symbols", default="SBER")
    parser.add_argument("--profile", choices=("all", "history", "quote", "combined", "lifecycle"), default="all")
    parser.add_argument("--server-log")
    args = parser.parse_args()
    endpoint = urllib.parse.urlsplit(args.base_url)
    if endpoint.scheme != "http" or endpoint.hostname not in {"127.0.0.1", "localhost", "::1"}:
        parser.error("base URL must target the local HTTP service")
    args.symbols = [symbol.strip().upper() for symbol in args.symbols.split(",") if symbol.strip()]
    if len(args.symbols) == 0 or args.duration_seconds <= 0:
        parser.error("duration must be positive and at least one symbol is required")
    print("profile=10_clients target_total_rps=10 body_fully_read=true", flush=True)
    if args.profile == "lifecycle":
        if not run_lifecycle(args, "SPBEX"):
            raise SystemExit("SPBEX history cache lifecycle verification failed")
        return
    modes = ("history", "quote", "combined") if args.profile == "all" else (args.profile,)
    for mode in modes:
        run_stage(args, mode)


if __name__ == "__main__":
    main()
