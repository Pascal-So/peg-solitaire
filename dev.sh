#!/usr/bin/env bash
set -eu

( trap 'kill 0' SIGINT; \
  bash -c 'cd frontend; CARGO_TARGET_DIR=../target-trunk trunk serve --dist ../dist --address 0.0.0.0' & \
  bash -c 'cd server; cargo watch -- cargo run' )
