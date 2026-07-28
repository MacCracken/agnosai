.PHONY: check fmt clippy test test-all bench bench-history audit deny vet fuzz coverage doc build release clean

# Run all CI checks locally
check: fmt clippy test-all audit deny doc

# Format check
fmt:
	cargo fmt --all -- --check

# Lint (zero warnings)
clippy:
	cargo clippy --all-features --all-targets -- -D warnings

# Run core tests
test:
	cargo test

# Run tests with all features
test-all:
	cargo test --all-features

# Run benchmarks (all benchmark files)
bench:
	cargo bench --all-features

# Run benchmarks and append to bench-history.csv
bench-history:
	./scripts/bench-history.sh

# Security audit
audit:
	cargo audit

# Supply-chain / license check
deny:
	cargo deny check

# Dependency audit chain
vet:
	cargo vet

# Fuzz all targets (30s each)
fuzz:
	@cd fuzz && for target in $$(cargo +nightly fuzz list 2>/dev/null); do \
		echo "=== Fuzzing $$target ==="; \
		cargo +nightly fuzz run "$$target" -- -max_total_time=30 -max_len=4096 || exit 1; \
	done

# Code coverage
coverage:
	cargo tarpaulin --all-features --skip-clean --fail-under 55

# Generate documentation (warnings as errors)
doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# Build release binary
build:
	cargo build --release --all-features

# Build and package release artifact
release:
	@VERSION=$$(cat VERSION | tr -d '[:space:]'); \
	cargo build --release --all-features; \
	tar czf "agnosai-server-$${VERSION}-linux-amd64.tar.gz" -C target/release agnosai-server; \
	sha256sum "agnosai-server-$${VERSION}-linux-amd64.tar.gz" > "agnosai-server-$${VERSION}-linux-amd64.tar.gz.sha256"; \
	echo "Packaged agnosai-server-$${VERSION}-linux-amd64.tar.gz"

# Clean build artifacts
clean:
	cargo clean
	rm -f agnosai-server-*.tar.gz agnosai-server-*.sha256
