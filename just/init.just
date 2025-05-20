set windows-shell := ["powershell.exe", "-c"]

[group("Init")]
init: install-devtools
    @echo "Run {{BOLD + YELLOW}}install-extra-devtools{{NORMAL}} for some adittional productivity tools that fit into the existent workflow"


# Trunk is used to serve the app with a webserver, cargo-dist is used to generate and update workflows for distributing installers for various platforms
[group("Init"), doc("Install the required tools for performing all dev tasks for the project")]
install-devtools:
    cargo install trunk --locked
    # Use the uv fork that is still maintained
    cargo install cargo-dist --locked --git https://github.com/astral-sh/cargo-dist.git --tag v0.28.5
    cargo install typos-cli --locked
    cargo install cargo-audit --locked

# Install nice-to-have devtools
[group("Init")]
install-extra-devtools:
    cargo install cargo-nextest --locked
    cargo install cargo-limit --locked
    cargo install bacon --locked
