import 'just/default_cmd.just'
mod ci 'just/ci.just'

PROJECT_NAME := "logviewer-rs"

alias l := lint
alias c := check
alias f := fmt
alias t := test

@_default:
    just --list --no-aliases

init: install-devtools
    echo "Run 'install-extra-devtools' for some adittional productivity tools that fit into the existent workflow"

# Check if it compiles without compiling
[no-exit-message]
check *ARGS:
    cargo {{check}} {{ ARGS }}

# Get trunk: https://trunkrs.dev/guide/introduction.html
[doc("serve as a local webserver with hot reloading and logging enabled (requires trunk)")]
serve *ARGS:
    RUST_LOG=debug trunk serve {{ARGS}}

# Run as a native app with logging enabled
[no-exit-message]
run *ARGS:
    RUST_LOG=debug cargo {{run}} {{ARGS}}

# Run tests
[no-exit-message]
test *ARGS="--workspace":
    cargo {{test}} {{ARGS}}

# Lint
[no-exit-message]
lint *ARGS: && fmt
    cargo {{clippy}} {{ARGS}}
    typos

[no-exit-message]
fmt *ARGS:
    cargo fmt --all -- {{ARGS}}

# Build the documentation (use `--open` to open in the browser)
[no-exit-message]
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

# Trunk is used to serve the app with a webserver, cargo-dist is used to generate and update workflows for distributing installers for various platforms
[doc("Install the required tools for performing all dev tasks for the project")]
install-devtools:
    cargo install trunk --locked
    cargo install cargo-dist --locked
    cargo install typos-cli
    cargo install cargo-audit

# Install nice-to-have devtools
install-extra-devtools:
    cargo install cargo-nextest --locked
    cargo install cargo-limit --locked


