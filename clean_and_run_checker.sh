#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
cd "$repo_root"

cargo clean
cargo build

./target/debug/api-bypass ./tests/get/src/main.rs --entry main --domain interval --widening_delay 5 --narrowing_iteration 5
