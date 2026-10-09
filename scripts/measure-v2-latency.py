#!/usr/bin/env python3
"""Measure each v2 provider route in its own paced 10-client workload."""

import argparse
import concurrent.futures
import http.client
import math
import time
import urllib.parse


ROUTES = (
    ("moex-history", "/v2/history/moex/SBER"),
    ("moex-quote", "/v2/quote/moex/SBER"),
    ("spbex-history", "/v2/history/spbex/SBER"),
    ("spbex-quote", "/v2/quote/spbex/SBER"),
    ("cbr-history", "/v2/history/cbr/USD"),
    ("cbr-quote", "/v2/quote/cbr/USD"),
)
CLIENTS = 10
REQUESTS_PER_SECOND = 10


def request_once(base_url, path, client_id):
    started = time.perf_counter()
    endpoint = urllib.parse.urlsplit(base_url)
    connection = http.client.HTTPConnection(endpoint.hostname, endpoint.port, timeout=60)
    try:
        connection.request("GET", path, headers={"X-Benchmark-Client": str(client_id)})
        response = connection.getresponse()
        response.read()
        return time.perf_counter() - started, response.status, None
    except Exception as error:  # Report transport failures separately from successful latency.
        return time.perf_counter() - started, None, str(error)
    finally:
        connection.close()


def run_profile(base_url, name, path, duration_seconds):
    slots = duration_seconds * REQUESTS_PER_SECOND
    started = time.monotonic()
    pending = {}
    active_clients = set()
    results = []
    issued = 0
    skipped = 0
    next_client_id = 0

    with concurrent.futures.ThreadPoolExecutor(max_workers=CLIENTS) as pool:
        for slot in range(slots):
            scheduled_at = started + slot / REQUESTS_PER_SECOND
            delay = scheduled_at - time.monotonic()
            if delay > 0:
                time.sleep(delay)
            for future in [item for item in pending if item.done()]:
                results.append(future.result())
                active_clients.remove(pending.pop(future))
            available = [
                (next_client_id + offset) % CLIENTS
                for offset in range(CLIENTS)
                if (next_client_id + offset) % CLIENTS not in active_clients
            ]
            if not available:
                skipped += 1
                continue
            client_id = available[0]
            next_client_id = (client_id + 1) % CLIENTS
            future = pool.submit(request_once, base_url, path, client_id)
            pending[future] = client_id
            active_clients.add(client_id)
            issued += 1
        for future, _client_id in list(pending.items()):
            results.append(future.result())

    successful = [latency for latency, status, _ in results if status is not None and 200 <= status < 300]
    errors = [result for result in results if result[1] is None or not 200 <= result[1] < 300]
    latencies = sorted(successful)
    p95 = latencies[max(0, math.ceil(len(latencies) * 0.95) - 1)] if latencies else None
    under_one_second = sum(latency < 1.0 for latency in successful)
    pass_latency = bool(successful) and under_one_second / len(successful) >= 0.95
    arrival_ok = issued == slots and skipped == 0
    error_rate = len(errors) / len(results) if results else 1.0
    print(
        f"profile={name} scheduled={slots} issued={issued} skipped={skipped} "
        f"successful={len(successful)} errors={len(errors)} clients={CLIENTS} "
        f"target_rps={REQUESTS_PER_SECOND} p95_seconds="
        f"{p95:.6f}" if p95 is not None else
        f"profile={name} scheduled={slots} issued={issued} skipped={skipped} "
        f"successful=0 errors={len(errors)} clients={CLIENTS} "
        f"target_rps={REQUESTS_PER_SECOND} p95_seconds=unavailable",
        flush=True,
    )
    print(
        f"profile={name} under_one_second={under_one_second}/{len(successful)} "
        f"error_rate={error_rate:.4f} arrival_schedule_valid={str(arrival_ok).lower()} "
        f"latency_target_met={str(pass_latency).lower()}",
        flush=True,
    )
    for _latency, status, error in errors[:10]:
        print(f"profile={name} error_status={status} detail={error}", flush=True)
    return arrival_ok and pass_latency


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-url", default="http://127.0.0.1:18080")
    parser.add_argument("--duration-seconds", type=int, default=55)
    args = parser.parse_args()
    endpoint = urllib.parse.urlsplit(args.base_url)
    if endpoint.scheme != "http" or endpoint.hostname not in {"127.0.0.1", "localhost", "::1"}:
        parser.error("base URL must target the local HTTP service")
    if not endpoint.port or args.duration_seconds <= 0:
        parser.error("base URL must include a port and duration must be positive")

    print(
        f"workload=separate-v2-route-profiles clients={CLIENTS} "
        f"requests_per_second={REQUESTS_PER_SECOND} "
        f"duration_seconds_per_profile={args.duration_seconds}",
        flush=True,
    )
    passed = [
        run_profile(args.base_url, name, path, args.duration_seconds)
        for name, path in ROUTES
    ]
    if not all(passed):
        raise SystemExit("one or more v2 route profiles missed the arrival or latency target")


if __name__ == "__main__":
    main()
