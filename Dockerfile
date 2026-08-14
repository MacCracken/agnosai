# AgnosAI — Cyrius build.
#
# ⚠ This file used to be `FROM rust:1.89-bookworm` + `cargo build --release
# --features full --bin agnosai-server`. None of that works: there is no
# Cargo.toml, Cyrius has no feature flags, and the binary is `agnosai` — the
# Rust tree's `agnosai-server` name did not survive the port. It failed on its
# first RUN.
#
# The toolchain version is NOT hardcoded here. `cyrius.cyml`'s `cyrius = "X.Y.Z"`
# is the single source of truth, and the installer resolves it, exactly as
# .github/workflows/ci.yml does.

FROM debian:bookworm-slim AS builder
WORKDIR /build

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl git && \
    rm -rf /var/lib/apt/lists/*

COPY . .

# Pin-driven toolchain install — read the version out of the manifest so this
# image can never drift from what CI builds.
RUN CYRIUS_VERSION="$(grep '^cyrius = ' cyrius.cyml | head -1 | sed 's/cyrius = "\(.*\)"/\1/')" && \
    echo "Installing Cyrius ${CYRIUS_VERSION}" && \
    curl -sSf https://raw.githubusercontent.com/MacCracken/cyrius/main/scripts/install.sh | \
        CYRIUS_VERSION="$CYRIUS_VERSION" sh
ENV PATH="/root/.cyrius/bin:${PATH}"

# ⚠ `lib sync --full` FIRST. `cyrius deps` only overlays [deps.NAME] on top of
# lib/; against an empty lib/ it fails on the stdlib leaves each dep's
# dist/*.deps sidecar names.
RUN cyrius lib sync --full && \
    cyrius deps && \
    cyrius build src/main.cyr build/agnosai

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/build/agnosai /usr/local/bin/agnosai
EXPOSE 8080

# `RUST_LOG` keeps the oracle's spelling deliberately — the port reads the same
# variable so a deployment's existing configuration keeps working.
ENV RUST_LOG=info

# ⚠ Rate limiting is MOUNTED BY DEFAULT (100 req/s per client key, burst 200).
# Set AGNOSAI_RATE_LIMIT=0 to restore the oracle's unlimited wire, or tune with
# AGNOSAI_RATE_LIMIT / AGNOSAI_RATE_LIMIT_BURST. See docs/adr/021.

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s \
    CMD curl -fsS http://127.0.0.1:8080/health || exit 1

ENTRYPOINT ["agnosai"]
