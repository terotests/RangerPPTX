#!/usr/bin/env bash
# Compiles a Rust-syntax crate with Ranger for one target (default go):
#   bash gallery/pptx/tools/rs_check.sh [go|es6|cpp] [core|edit]   (from the Ranger root)
# core is src/lib.rs on its own; edit is edit/src/lib.rs, which takes core with it.
set -u
cd "$(cd "$(dirname "$0")/../../.." && pwd)"
t=${1:-go}
entry=gallery/pptx/tools/rs_check.rgr
if [ "${2:-core}" = edit ]; then entry=gallery/pptx/tools/rs_check_edit.rgr; fi
export RANGER_LIB=./compiler/Lang.rgr:./lib/stdops.rgr
log=$(node --max-old-space-size=8192 dist/rgrc.js -l=$t $entry -d=tmp/rs_check -o=rs_check -nodecli 2>&1)
echo "$log" | grep -A3 "\[FAIL\]" | head -200
if echo "$log" | grep -q "Compilation FAILED"; then exit 1; fi
