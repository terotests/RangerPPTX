#!/usr/bin/env bash
# Compile one interop test to JavaScript and to C++ and run both — the same
# two-target run Ranger's gallery/odp and gallery/ooxml runners do, because a
# string is code units on one target and bytes on the other.
#
#   bash gallery/pptx/interop/run_two_targets.sh <Test>.rgr
set -euo pipefail
cd "$(cd "$(dirname "$0")/../../.." && pwd)"

export RANGER_LIB=./compiler/Lang.rgr:./lib/stdops.rgr
SRC=gallery/pptx/interop/$1
NAME=$(basename "$1" .rgr)
OUT=tmp/pptx-interop/$NAME
mkdir -p "$OUT" gallery/pptx/bin

echo "==> JavaScript"
node dist/rgrc.js -es6 "$SRC" -d=gallery/pptx/bin -o="$NAME.js" -nodecli > "$OUT/js.log" 2>&1 || {
  tail -20 "$OUT/js.log"; echo "Ranger -> JS failed" >&2; exit 1; }
if grep -q '\[FAIL\]' "$OUT/js.log"; then
  grep -A2 '\[FAIL\]' "$OUT/js.log" | head -20
  echo "Ranger -> JS failed" >&2
  exit 1
fi
node "gallery/pptx/bin/$NAME.js" | tee "$OUT/js.out"
grep -q "ALL PASS" "$OUT/js.out" || { echo "JavaScript run failed" >&2; exit 1; }

CXX=""
for cc in g++ clang++; do
  if command -v "$cc" >/dev/null 2>&1; then CXX="$cc"; break; fi
done
if [ -z "$CXX" ]; then
  echo
  echo "==> C++  SKIPPED — no g++ or clang++ on PATH."
  exit 0
fi

echo
echo "==> C++ ($CXX)"
node dist/rgrc.js -l=cpp "$SRC" -nodecli -d="$OUT" -o="$NAME.cpp" > "$OUT/cpp.log" 2>&1 || {
  tail -20 "$OUT/cpp.log"; echo "Ranger -> C++ failed" >&2; exit 1; }
if grep -q '\[FAIL\]' "$OUT/cpp.log"; then
  grep -A2 '\[FAIL\]' "$OUT/cpp.log" | head -20
  echo "Ranger -> C++ failed" >&2
  exit 1
fi
cp gallery/invaders/variant.hpp "$OUT/variant.hpp"
"$CXX" -std=c++17 -I "$OUT" -o "$OUT/$NAME" "$OUT/$NAME.cpp"
"$OUT/$NAME" | tee "$OUT/cpp.out"
grep -q "ALL PASS" "$OUT/cpp.out" || { echo "C++ run failed" >&2; exit 1; }
