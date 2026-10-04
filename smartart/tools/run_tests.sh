#!/usr/bin/env bash
# The SmartArt suites on all three targets the engine has to run on:
# JavaScript (the editor), C++ (the native builds) and Go (Sliqtly's MCP
# server, which lays decks out with the editor's own model). A suite passes
# only if all three print ALL PASS. A missing C++ or Go toolchain is a failure,
# not a skip: a target nobody ran is a target nobody knows about.
#
#   bash gallery/pptx/smartart/tools/run_tests.sh            # every suite
#   bash gallery/pptx/smartart/tools/run_tests.sh SaDataTest # one
set -uo pipefail
cd "$(cd "$(dirname "$0")/../../../.." && pwd)"

export RANGER_LIB=./compiler/Lang.rgr:./lib/stdops.rgr
TESTS=gallery/pptx/smartart/tests
OUT=tmp/smartart
mkdir -p "$OUT"

if [ $# -gt 0 ]; then
  SUITES=("$@")
else
  SUITES=()
  for f in "$TESTS"/*Test.rgr; do SUITES+=("$(basename "$f" .rgr)"); done
fi

compile() { # target src outdir outfile
  local log
  log=$(node dist/rgrc.js "$1" "$2" -d="$3" -o="$4" -nodecli 2>&1)
  if [ $? -ne 0 ] || echo "$log" | grep -q "Compilation FAILED"; then
    echo "$log" | grep -A4 "\[FAIL\]" | head -30
    return 1
  fi
  [ -f "$3/$4" ]
}

check() { # name output-file
  if grep -q "^ALL PASS" "$2"; then
    echo "    $1: $(grep '^passed' "$2")"
    return 0
  fi
  grep "FAIL" "$2" | head -20
  echo "    $1: FAILED" >&2
  return 1
}

status=0
for s in "${SUITES[@]}"; do
  src="$TESTS/$s.rgr"
  dir="$OUT/$s"
  mkdir -p "$dir/go"
  echo "== $s"

  if compile -es6 "$src" "$dir" "$s.js" && node "$dir/$s.js" > "$dir/js.out" 2>&1; then
    check "JavaScript" "$dir/js.out" || status=1
  else
    echo "    JavaScript: did not compile or crashed" >&2; tail -5 "$dir/js.out" 2>/dev/null; status=1
  fi

  if compile -l=cpp "$src" "$dir" "$s.cpp"; then
    cp gallery/invaders/variant.hpp "$dir/variant.hpp"
    if g++ -std=c++17 -O1 -I "$dir" -o "$dir/$s.bin" "$dir/$s.cpp" 2> "$dir/cxx.log" && "$dir/$s.bin" > "$dir/cpp.out" 2>&1; then
      check "C++" "$dir/cpp.out" || status=1
    else
      echo "    C++: did not build or crashed" >&2; head -20 "$dir/cxx.log"; status=1
    fi
  else
    echo "    C++: did not compile" >&2; status=1
  fi

  if compile -l=go "$src" "$dir/go" "$s.go"; then
    if (cd "$dir/go" && go run "$s.go") > "$dir/go.out" 2>&1; then
      check "Go" "$dir/go.out" || status=1
    else
      echo "    Go: did not build or crashed" >&2; tail -20 "$dir/go.out"; status=1
    fi
  else
    echo "    Go: did not compile" >&2; status=1
  fi
done
exit $status
