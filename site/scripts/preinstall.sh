#!/bin/bash

# This script is necessary because the querydown-js bindings packages is
# generated -- not committed directly to git. This script ensures that the
# bindings package is generated before NPM install is run. Without this, NPM
# install will fail because it will not find the bindings package.
#
# To build the bindings packge, we need `wasm-pack` and a rust toolchain.
#
# https://docs.netlify.com/configure-builds/manage-dependencies/#rust

set -u
set -e

echo "Checking for cargo"
if ! cargo --version; then
    echo "Checking for rustup"
    if ! command -v rustup; then
        echo "ERROR: You need to have rustup installed first."
        echo "See https://www.rust-lang.org/tools/install"
        echo ""
        exit 1
    fi
    echo "cargo is not installed. Installing..."
    rustup install stable
    rustup default stable
    install_status=$?
    if [ $install_status -ne 0 ]; then
        echo "Failed to install rustc."
        exit $install_status
    fi
fi

echo "Checking for wasm-pack"
if ! command -v wasm-pack; then
    echo "wasm-pack is not installed. Installing..."
    npm i -g wasm-pack
    install_status=$?
    if [ $install_status -ne 0 ]; then
        echo "Failed to install wasm-pack."
        exit $install_status
    fi
fi

wasm-pack build ../bindings/js

exit 0
