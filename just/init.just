# Trunk is used to serve the app with a webserver, cargo-dist is used to generate and update workflows for distributing installers for various platforms
[group("Init"), doc("Install the required tools for performing all dev tasks for the project")]
install-devtools:
    cargo install trunk --locked
    cargo install cargo-dist --locked
    cargo install typos-cli --locked
    cargo install cargo-audit --locked

# Install nice-to-have devtools
[group("Init")]
install-extra-devtools:
    cargo install cargo-nextest --locked
    cargo install cargo-limit --locked
    cargo install bacon --locked

[group("Init")]
apt-install-hdf5-header:
    sudo apt install libhdf5-dev