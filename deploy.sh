#!/usr/bin/env bash

set -eu

if [ ! -f Cargo.toml ]; then
    echo "Cargo.toml not found in this directory"
    exit 1
fi

trunk --config ./Trunk.deploy.toml build --release
rsync -av dist/ gegubiha:www/codelis/root/pegsolitaire/
