init: install-devtools install-hdf5-headers
    echo "Run 'install-extra-devtools' for some adittional productivity tools that fit into the existent workflow"

# Trunk is used to serve the app with a webserver, cargo-dist is used to generate and update workflows for distributing installers for various platforms
[doc("Install the required tools for performing all dev tasks for the project")]
install-devtools:
    cargo install trunk --locked
    cargo install cargo-dist --locked
    cargo install typos-cli --locked
    cargo install cargo-audit --locked

# Install nice-to-have devtools
install-extra-devtools:
    cargo install cargo-nextest --locked
    cargo install cargo-limit --locked
    cargo install bacon --locked

[linux]
apt-install-hdf5-headers:
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
set-github-hdf5-env-vars VERSION HDF5_MSI_INSTALLER_PATH:
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
