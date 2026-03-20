.PHONY: check fmt clippy test audit deny build release doc clean

# Run all CI checks locally
check: fmt clippy test audit deny

# Format check
fmt:
	cargo fmt --all -- --check

# Lint (zero warnings)
clippy:
	cargo clippy --workspace --all-targets -- -D warnings

# Run test suite
test:
	cargo test --workspace

# Security audit
audit:
	cargo audit

# Supply-chain / license check
deny:
	cargo deny check

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
