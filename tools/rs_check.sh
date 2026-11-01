#!/usr/bin/env bash
# Compiles the crate (src/lib.rs) with Ranger for one target (default go):
#   bash gallery/pptx/tools/rs_check.sh [go|es6|cpp]   (from the Ranger root)
set -u
cd "$(cd "$(dirname "$0")/../../.." && pwd)"
t=${1:-go}
export RANGER_LIB=./compiler/Lang.rgr:./lib/stdops.rgr
log=$(node --max-old-space-size=8192 dist/rgrc.js -l=$t gallery/pptx/tools/rs_check.rgr -d=tmp/rs_check -o=rs_check -nodecli 2>&1)
echo "$log" | grep -A3 "\[FAIL\]" | head -200
if echo "$log" | grep -q "Compilation FAILED"; then exit 1; fi
