
@_default:
    just --list --no-aliases

# Get trunk: https://trunkrs.dev/guide/introduction.html
[doc("serve as a local webserver with hot reloading and logging enabled (requires trunk)")]
serve *ARGS:
    RUST_LOG=debug trunk serve {{ARGS}}

# Run as a native app with logging enabled
run *ARGS:
    RUST_LOG=debug cargo run {{ARGS}}

# Run tests
test *ARGS:
    cargo test

# About nextest: https://nexte.st/
[doc("Run the tests with nextest (faster and much more features)")]
ntest *ARGS:
    cargo nextest run

# Lint
lint *ARGS:
    cargo clippy {{ARGS}}

format *ARGS:
    cargo fmt --all -- {{ARGS}}

# Install nice-to-have devtools
install-extra-devtools:
    cargo install cargo-nextest --locked
    cargo install trunk --locked 

