set windows-shell := ["powershell.exe", "-c"]

[group("Init")]
init: install-devtools
    @echo "Run {{BOLD + YELLOW}}install-extra-devtools{{NORMAL}} for some adittional productivity tools that fit into the existent workflow"
    @echo "Run {{BOLD + YELLOW}}apt-install-hdf5-header{{NORMAL}} to get HDF5 headers for developing HDF5 features on linux"


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

[windows]
msi-install-hdf5-headers:
    #!/usr/bin/env bash
    set -eo pipefail
    VERSION="1.14.0"
    DL_URL="https://support.hdfgroup.org/ftp/HDF5/releases/hdf5-1.14/hdf5-${VERSION}/bin/windows/hdf5-${VERSION}-Std-win10_64-vs16.zip"
    # Download and extract HDF5
    curl -L -o hdf5.zip "${DL_URL}"
    unzip hdf5.zip
    if [[ "${GITHUB_ACTIONS}" == "true" ]]; then
        just init::set-github-hdf5-env-vars "${VERSION}"
    fi


[windows]
set-github-hdf5-env-vars VERSION:
    echo "HDF5_DIR=C:\\Program Files\\HDF_Group\\HDF5\\{{VERSION}}" >> $GITHUB_ENV
    echo "C:\\Program Files\\HDF_Group\\HDF5\\{{VERSION}}\\bin" >> $GITHUB_PATH

[macos]
brew-install-hdf5-headers:
    brew install hdf5

[linux]
install-hdf5-headers: apt-install-hdf5-headers

[macos]
install-hdf5-headers: brew-install-hdf5-headers

[windows]
install-hdf5-headers: msi-install-hdf5-headers