import 'just/mod.just'
# CI only recipes, `just -l ci` to list them
mod ci 'just/ci.just'
# init only recipes, `just -l init` to list them
mod init 'just/init.just'

set windows-shell := ["powershell.exe", "-c"]

PROJECT_NAME := "plotinator3000"

alias t := test
alias l := lint
alias fmt := format
alias f := format
alias d := doc
alias r := run
alias start := run
alias s := run
alias c := check
alias ca := check-all

# Achieve higher verbosity in run command e.g. by running "RUST_LOG=debug just run --release"
export RUST_LOG := env_var_or_default("RUST_LOG", "info")
# Ensure it doesn't run the updating process during development
export PLOTINATOR_BYPASS_UPDATES := env_var_or_default("PLOTINATOR_BYPASS_UPDATES", "true")

@_default:
    just --list

[doc("Checks both native and wasm"), group("Check"), no-exit-message]
check-all: check check-wasm

[doc("Quickly check if it compiles without compiling (native target)"), group("Check"), no-exit-message]
check *ARGS:
    cargo {{check}} {{ ARGS }}

[group("Web"), group("Check"), doc("Quickly check if the WASM target compiles without compiling"), no-exit-message]
check-wasm: (check "--target wasm32-unknown-unknown")

# Get trunk: https://trunkrs.dev/guide/introduction.html
[group("Web"), doc("serve as a local webserver with hot reloading and logging enabled (requires trunk)")]
serve *ARGS:
    trunk serve {{ARGS}}

# Run as a native app with logging enabled
[group("Run"), no-exit-message]
run *ARGS:
    cargo {{run}} {{ARGS}}

# Run tests
[group("Check"), no-exit-message]
test *ARGS="--workspace":
    cargo {{test}} {{ARGS}}

# Lint native & web, check typos and format
[group("Check"), no-exit-message]
lint: clippy-native clippy-wasm && format
    typos

[doc("Clippy linting targeting native"), group("Check"), no-exit-message]
clippy-native: (clippy "--workspace --tests -- -D warnings")

[group("Web"), group("Check"), doc("Clippy linting targeting WASM"), no-exit-message]
clippy-wasm:
    CLIPPY_CONF_DIR="`pwd`/lint/wasm/clippy.toml" \
    just clippy "--workspace --tests --target wasm32-unknown-unknown -- -D warnings"

[private, no-exit-message]
clippy *ARGS:
    cargo {{clippy}} {{ARGS}}

[group("Misc"), no-exit-message]
build *ARGS:
    cargo {{build}} {{ARGS}}

# Format code
[group("Misc"), no-exit-message]
format *ARGS:
    cargo fmt --all -- {{ARGS}}

# Build the documentation (use `--open` to open in the browser)
[group("Misc"), no-exit-message]
doc *ARGS:
    cargo {{doc}} {{ ARGS }}

# Update the dependencies
[group("Dependencies"), no-exit-message]
update:
    cargo update

# Audit Cargo.lock files for crates containing security vulnerabilities
[group("Dependencies"), no-exit-message]
audit *ARGS:
    cargo audit {{ ARGS }}

[group("Profiling")]
run-profiling *ARGS:
    cargo install puffin_viewer --locked
    cargo {{run}} --features profiling -- {{ARGS}}

# Requires firebase CLI and access to MKI firebase account
[group("Web")]
firebase-deploy:
    trunk clean
    trunk build --release
    firebase deploy
