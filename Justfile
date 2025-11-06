import 'just/mod.just'
# CI only recipes, `just -l ci` to list them
mod ci 'just/ci.just'
# init only recipes, `just -l init` to list them
mod init 'just/init.just'

set windows-shell := ["powershell.exe", "-c"]

PROJECT_NAME := "plotinator3000"

alias t := test
alias l := lint
alias lint-native := clippy-native
alias ln := clippy-native
alias fmt := format
alias f := format
alias d := doc
alias r := run
alias b := build
alias start := run
alias s := run
alias c := check

# Achieve higher verbosity in run command e.g. by running "RUST_LOG=plotinator3000=debug just run --release"
export RUST_LOG := env_var_or_default("RUST_LOG", "error,plotinator=debug")
# Ensure it doesn't run the updating process during development
export PLOTINATOR_BYPASS_UPDATES := env_var_or_default("PLOTINATOR_BYPASS_UPDATES", "true")

@_default:
    just --list

[doc("Quickly check if it compiles without compiling (native target)"), group("Check"), no-exit-message]
check *ARGS="--workspace":
    cargo {{check}} {{ ARGS }}


[group("Web"), doc("serve as a local webserver with hot reloading and logging enabled (requires trunk)")]
serve *ARGS:
    trunk serve {{ARGS}}

# Run as a native app with logging enabled
[group("Run"), no-exit-message]
run *ARGS:
    cargo {{run}} {{ARGS}}

# Run all tests
[group("Check"), no-exit-message]
test *ARGS="--workspace":
    cargo {{test}} {{ARGS}}

# Lint, check typos, and format
[group("Check"), no-exit-message]
lint: clippy-native && format
    typos

[doc("Clippy linting targeting native"), group("Check"), no-exit-message]
clippy-native: (clippy "--workspace --tests -- -D warnings")

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
    cargo {{run}} --features profiling {{ARGS}}

[group("Profiling")]
run-log-time *ARGS:
    cargo {{run}} --features log_time {{ARGS}}

# Requires firebase CLI and access to MKI firebase account
[group("Web")]
firebase-deploy:
    trunk clean
    trunk build --release
    firebase deploy
