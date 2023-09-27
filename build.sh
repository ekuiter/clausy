#!/bin/bash
# This script sets up clausy in the bin/ directory, which includes all dependencies necessary for distribution.
set -e

has-command() {
    command -v "$1" &> /dev/null
}

require-command() {
    if ! has-command "$1"; then
        echo "Required command $1 could not be found, please install manually." >&2
        exit 1
    fi
}

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

if [[ ! -f bin/io.jar ]]; then
    require-command java
    io/gradlew -p io shadowJar
fi

require-command cc
cargo build --release
cp target/release/clausy bin/clausy