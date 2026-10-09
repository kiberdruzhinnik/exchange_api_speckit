#!/usr/bin/env python3
"""Serve repository provider fixtures as local upstreams for the v2 latency profile."""

import argparse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit


ROOT = Path(__file__).resolve().parents[1] / "tests" / "fixtures"
FIXTURES = {
    "/iss/securities/SBER.json": ROOT / "moex/security-description.json",
    "/iss/engines/stock/markets/shares/boards/TQBR/securities/SBER.json": ROOT / "moex/security-tqbr.json",
    "/iss/history/engines/stock/markets/shares/securities/SBER.json": ROOT / "moex/history-page.json",
    "/iss/engines/stock/markets/shares/securities/SBER/trades.json": ROOT / "moex/trades-latest.json",
    "/spbex/api/reader/marketdata/charts/chistory": ROOT / "spbex/history.json",
    "/cbr/scripts/XML_valFull.asp": ROOT / "cbr/currencies.xml",
    "/cbr/scripts/XML_dynamic.asp": ROOT / "cbr/history.xml",
}


class FixtureHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        path = urlsplit(self.path).path
        if path == "/cbr/scripts/XML_daily.asp":
            name = "current-currencies.xml" if "d=0" in urlsplit(self.path).query else "latest.xml"
            fixture = ROOT / "cbr" / name
        else:
            fixture = FIXTURES.get(path)
        if fixture is None:
            self.send_error(404)
            return
        body = fixture.read_bytes()
        self.send_response(200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, _format, *_args):
        pass


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=18081)
    args = parser.parse_args()
    server = ThreadingHTTPServer(("127.0.0.1", args.port), FixtureHandler)
    print(f"fixture upstream server listening on http://127.0.0.1:{args.port}", flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
