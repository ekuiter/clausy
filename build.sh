#!/bin/bash
# This script sets up clausy in the bin/ directory, which includes all dependencies necessary for distribution.
# It can also be used to run tests or generate documentation.
set -e

ACTION=${1:-build}

has-command() {
    command -v "$1" &> /dev/null
}

require-command() {
    if ! has-command "$1"; then
        echo "Required command $1 could not be found, please install manually." >&2
        exit 1
    fi
}

if [[ $ACTION == build ]]; then
    if ! has-command cargo; then
        require-command curl
        curl https://sh.rustup.rs -sSf | sh
        # shellcheck disable=1091
        source "$HOME/.cargo/env"
    fi

    mkdir -p bin

    if [[ ! -f bin/d4 ]]; then
        require-command curl
        curl https://github.com/ekuiter/torte/raw/main/docker/solver/model-counting-competition-2022/d4 -Lo bin/d4
        chmod +x bin/d4
    fi

    if [[ ! -f bin/kissat_MAB-HyWalk ]]; then
        require-command curl
        curl https://github.com/ekuiter/torte/raw/main/docker/solver/other/kissat_MAB-HyWalk -Lo bin/kissat_MAB-HyWalk
        chmod +x bin/kissat_MAB-HyWalk
    fi

    if [[ ! -f bin/io.jar ]]; then
        require-command java
        io/gradlew -p io shadowJar
    fi

    require-command cc
    cargo build --release
    cp target/release/clausy bin/clausy
elif [[ $ACTION == test ]]; then
    cargo test
elif [[ $ACTION == doc ]]; then
    cargo doc --no-deps --open
elif [[ $ACTION == doc-live ]]; then
    # sudo apt-get update
    # sudo apt-get install -y inotify-tools nodejs npm
    # npm install -g browser-sync
    require-command inotifywait browser-sync
    while inotifywait -re close_write,moved_to,create src; do
        cargo doc --no-deps
    done &
    (cd target/doc; browser-sync start --server --files "*.html")
else
    echo "Invalid usage." >&2
fi