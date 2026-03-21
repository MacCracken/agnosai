.PHONY: check fmt clippy test audit deny vet bench fuzz coverage build release doc clean

# Run all CI checks locally
check: fmt clippy test audit deny vet

# Format check
fmt:
	cargo fmt --all -- --check

# Lint (zero warnings, allow missing-docs for now)
clippy:
	cargo clippy --workspace --all-targets -- -D warnings -A missing-docs

# Run test suite
test:
	cargo test --workspace

# Security audit
audit:
	cargo audit

# Supply-chain / license check
deny:
	cargo deny check

# Dependency audit chain
vet:
	cargo vet

# Run benchmarks
bench:
	cargo bench --bench serde_types --bench scheduler --bench scoring -- --noplot

# Fuzz all targets (30s each)
fuzz:
	@cd fuzz && for target in $$(cargo +nightly fuzz list 2>/dev/null); do \
		echo "=== Fuzzing $$target ==="; \
		cargo +nightly fuzz run "$$target" -- -max_total_time=30 -max_len=4096 || exit 1; \
	done

# Coverage (fails if below 55%)
coverage:
	cargo tarpaulin --workspace --skip-clean --fail-under 55

# Build release binary
build:
	cargo build --release

# Build and package release artifact
release:
	@VERSION=$$(cat VERSION | tr -d '[:space:]'); \
	cargo build --release; \
	tar czf "agnosai-server-$${VERSION}-linux-amd64.tar.gz" -C target/release agnosai-server; \
	sha256sum "agnosai-server-$${VERSION}-linux-amd64.tar.gz" > "agnosai-server-$${VERSION}-linux-amd64.tar.gz.sha256"; \
	echo "Packaged agnosai-server-$${VERSION}-linux-amd64.tar.gz"

# Generate documentation
doc:
	cargo doc --no-deps --workspace

# Clean build artifacts
clean:
	cargo clean
	rm -f agnosai-server-*.tar.gz agnosai-server-*.sha256
