#!/bin/zsh
set -euo pipefail
PACKAGE_DIR="${0:A:h}"
"$PACKAGE_DIR/monitor-keys" install
print '\nReady: F1 = USB-C, F2 = HDMI, F3 = DisplayPort. On MX Keys Mini, hold Fn as needed.'
