#!/bin/zsh
set -euo pipefail
SOURCE_DIR="${0:A:h:h}"
cd "$SOURCE_DIR"
if [[ "$(uname -m)" != arm64 ]]; then
  print -u2 'Build on an Apple Silicon Mac using a native terminal.'
  exit 1
fi
MACOSX_DEPLOYMENT_TARGET=26.0 cargo build --release --locked
cp "${CARGO_TARGET_DIR:-target}/release/monitor-keys" ../monitor-keys
codesign --force --sign - ../monitor-keys
codesign --verify --strict ../monitor-keys
print 'Built ../monitor-keys'
