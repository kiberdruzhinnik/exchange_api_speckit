FROM --platform=$BUILDPLATFORM rust:1.89-bookworm AS builder
ARG TARGETARCH
WORKDIR /app
RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends cmake nasm perl; \
    case "$TARGETARCH" in \
      amd64) cross_gcc=gcc-x86-64-linux-gnu; cross_gxx=g++-x86-64-linux-gnu; rust_target=x86_64-unknown-linux-gnu ;; \
      arm64) cross_gcc=gcc-aarch64-linux-gnu; cross_gxx=g++-aarch64-linux-gnu; rust_target=aarch64-unknown-linux-gnu ;; \
      *) echo "unsupported target architecture: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    apt-get install -y --no-install-recommends "$cross_gcc" "$cross_gxx"; \
    rustup target add "$rust_target"; \
    apt-get clean; \
    rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN set -eux; \
    case "$TARGETARCH" in \
      amd64) rust_target=x86_64-unknown-linux-gnu; export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc CXX_x86_64_unknown_linux_gnu=x86_64-linux-gnu-g++ AR_x86_64_unknown_linux_gnu=x86_64-linux-gnu-ar ;; \
      arm64) rust_target=aarch64-unknown-linux-gnu; export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++ AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar ;; \
      *) echo "unsupported target architecture: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    cargo build --release --locked --target "$rust_target"; \
    mkdir -p /out /var/lib/exchange-api; \
    cp "/app/target/$rust_target/release/exchange-api" /out/exchange-api; \
    touch /var/lib/exchange-api/.keep

FROM gcr.io/distroless/cc-debian13:nonroot
COPY --from=builder /out/exchange-api /exchange-api
COPY --from=builder --chown=65532:65532 /var/lib/exchange-api /var/lib/exchange-api
EXPOSE 8080
USER 65532:65532
ENTRYPOINT ["/exchange-api"]
