
@_default:
    just --list --no-aliases

# serve as a local webserver with hot reloading and logging enabled
serve *ARGS:
    RUST_LOG=debug trunk serve {{ARGS}}

# Run as a native app with logging enabled
run *ARGS:
    RUST_LOG=debug cargo run {{ARGS}}

test *ARGS:
    cargo test

# Run the tests with nextest (faster and much more features)
ntest *ARGS:
    cargo nextest run


install-extra-devtools:
