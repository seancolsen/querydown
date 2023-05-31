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

if ! command -v rustc &>/dev/null; then
    if ! command -v rustup &>/dev/null; then
        echo "ERROR: You need to have rustup installed first."
        echo "See https://www.rust-lang.org/tools/install"
        echo ""
        exit 1
    fi
    echo "rustc is not installed. Installing..."
    rustup toolchain install stable
    install_status=$?
    if [ $install_status -ne 0 ]; then
        echo "Failed to install rustc."
        exit $install_status
    fi
fi

if ! command -v wasm-pack &>/dev/null; then
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
