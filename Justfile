
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
    typos

fmt *ARGS:
    cargo fmt --all -- {{ARGS}}

# Trunk is used to serve the app with a webserver, cargo-dist is used to generate and update workflows for distributing installers for various platforms
[doc("Install the required tools for performing all dev tasks for the project")]
install-devtools:
    cargo install trunk --locked 
    cargo install cargo-dist --locked
    cargo install typos-cli

# Install nice-to-have devtools
install-extra-devtools:
    cargo install cargo-nextest --locked
    

[group("CI")]
ci-fmt: (fmt " --check")

[group("CI")]
ci-lint: (lint "--workspace --all-targets --all-features --  -D warnings -W clippy::all")

[group("CI")]
ci-test: (test "--lib")
