# Recipes as they are run in CI

export RUSTFLAGS := "-D warnings"
export RUSTDOCFLAGS := "-D warnings"
export RUST_LOG := "debug"

ci: lint check fmt test trunk audit dist-plan

# We only check the most common native linux target because that is the target we usually develop on,
# it will catch 99.99% of errors, AND we also build for MacOS and Windows in CI so this is just quick feedback
[doc("Check compilation for linux and WASM targets")]
check:
    cargo check --all-features --bins --target x86_64-unknown-linux-gnu
    cargo check --bin plotinator3000 --target wasm32-unknown-unknown

# Check formatting
fmt:
    cargo fmt --all -- --check

# Check clippy lints, treating warnings as errors, and check for typos
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::all
    typos

# Run all tests with the CI profile and include all ignored tests
test:
    cargo nextest run --workspace --profile ci --run-ignored=all

# Audit dependencies
audit:
    cargo audit

# Build WASM bundle
trunk:
    trunk build

# Check if distribution settings are OK
dist-plan:
    cargo dist plan
