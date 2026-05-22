#!/usr/bin/env bash

# This file builds a target crate with mir_wrapper for debugging.

set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"

usage() {
  cat <<'EOF'
Usage:
  target_crate_build.sh [DIRECTORY]

Description:
  Build DIRECTORY with this repository's mir_wrapper. If DIRECTORY is omitted,
  build $HOME.

Examples:
  ./target_crate_build.sh /path/to/crate
  MIR_WRAPPER_DUMP=/tmp/inv.json ./target_crate_build.sh ~/workspace
EOF
}

# Help
if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  return 0 2>/dev/null || exit 0
fi

# Default directory
target="${1:-$HOME}"

# Expand ~ manually (bash won't expand it inside quotes reliably in all cases)
if [[ "$target" == "~" ]]; then
  target="$HOME"
elif [[ "$target" == "~/"* ]]; then
  target="$HOME/${target#~/}"
fi

# Validate
if [[ ! -d "$target" ]]; then
  echo "Error: not a directory: $target" >&2
  return 2 2>/dev/null || exit 2
fi

# Change directory
cd "$target"

# Optional: show where we are
pwd

# build the target crate
rustup override set nightly-2025-01-10
rm -rf target
export RUSTC_WRAPPER="${RUSTC_WRAPPER:-$repo_root/target/debug/mir_wrapper}"
export MIR_WRAPPER_DUMP="${MIR_WRAPPER_DUMP:-/var/tmp/inv.json}"
export MIR_CHECKER_ARGS="${MIR_CHECKER_ARGS:-[\"--show_all_entries\"]}"

# env | grep RUSTC

cargo +nightly-2025-01-10 build -vv
