#!/usr/bin/env bash

set -eu

if [ ! -f Cargo.lock ]; then
    echo "Cargo.lock not found in this directory, move to the workspace root first"
    exit 1
fi

pushd frontend
trunk --config ./Trunk.deploy.toml build --release --dist ../dist
popd

rsync -av dist/ gegubiha:www/codelis/root/pegsolitaire/
