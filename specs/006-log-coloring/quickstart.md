# Quickstart: Configurable Log Coloring

## Configuration

Set `EXCHANGE_API_LOG_COLOR` before starting the service. Coloring is enabled
when the variable is absent or has any value other than `false`, `0`, `no`, or
`off`. Those four values disable coloring, case-insensitively. Restart the
service after changing the environment.

Example for a colorless terminal or log collector:

```sh
EXCHANGE_API_LOG_COLOR=off cargo run
```

To explicitly enable coloring:

```sh
EXCHANGE_API_LOG_COLOR=true cargo run
```

## Validation

Prerequisites: Rust toolchain specified by `rust-toolchain.toml`.

1. Run configuration unit tests with `cargo test config::tests`.
2. Verify configuration parsing for no setting, an enabled value, each disabling
      value in mixed case, and an unrecognized value.
3. Start the service once with `EXCHANGE_API_LOG_COLOR=off` and once with the
      variable unset. Capture standard output through graceful SIGINT shutdown.
      The disabled run must contain no ANSI escape sequences; the unset run must
      retain colored output.
4. Compare the startup, SIGINT receipt, and shutdown log messages in both runs
      to confirm their text, `INFO` severity, and ordering remain the same while
      only coloring changes.

The environment contract is documented in
[contracts/log-color-config.md](contracts/log-color-config.md).

## Release validation

The project constitution requires a final Linux amd64 image build, Semgrep
source analysis, and a Trivy scan of the built image before the feature is
complete. Resolve all Semgrep findings and every Trivy High or Critical finding
that has an available fix. Record scan results and any unavailable scan or fix
below.

Build the final image:

```sh
docker buildx build --platform linux/amd64 -t exchange-api:006-log-coloring --load .
```

Run Semgrep against the source tree:

```sh
docker run --rm -v "$PWD:/src" semgrep/semgrep:latest semgrep scan --config auto /src
```

Scan the built image with Trivy:

```sh
docker run --rm -v /var/run/docker.sock:/var/run/docker.sock aquasec/trivy:latest image --scanners vuln --severity HIGH,CRITICAL exchange-api:006-log-coloring
```

**Results (2026-10-10)**:

- `cargo test config::tests`: passed (5 tests).
- `cargo test --test log_color`: passed (2 tests).
- Convergence regression coverage: the subprocess test captures startup and
    graceful shutdown messages, checks `INFO` severity and ordering with colors
    enabled and disabled, and passes in both the focused test and full suite.
- `cargo test`: passed (all unit and integration tests).
- `cargo fmt -- --check`: passed.
- Linux amd64 build: `docker buildx build --platform linux/amd64 -t
    exchange-api:006-log-coloring --load .` completed successfully after the
    convergence test update; image `exchange-api:006-log-coloring` is available
    locally.
- Semgrep: scan completed successfully after the convergence test update; 0
    findings across 147 tracked files.
- Trivy: image scan completed successfully after the final build; 0 High or
    Critical vulnerabilities for Debian 13.7. The configured mirror returned an
    unavailable blob, and Trivy successfully downloaded its database from the
    fallback registry before scanning.
- Unavailable scans or fixes: none.
