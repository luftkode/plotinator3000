# Recipes as they are run in CI

export RUSTFLAGS := "-D warnings"
export RUSTDOCFLAGS := "-D warnings"
export RUST_LOG := "debug"

ci: lint check fmt test trunk audit dist-plan

check:
    cargo check --all-features --bins --target x86_64-unknown-linux-gnu
    cargo check --bin plotinator3000 --target wasm32-unknown-unknown

fmt:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets --all-features --  -D warnings -W clippy::all
    typos

test:
    cargo nextest run --workspace --profile ci --run-ignored=all

audit:
    cargo audit

trunk:
    trunk build

dist-plan:
    cargo dist plan
