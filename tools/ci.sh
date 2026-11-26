#!/usr/bin/env bash
# Runs RangerPPTX's checks from the root of a Ranger checkout that has this
# repository at gallery/pptx (and `npm ci` done, which fetches lib/evg and the
# other deps):
#
#   bash gallery/pptx/tools/ci.sh
#
# Every suite in tests/, the reader audit over the fixtures, the npm API's
# tests, and the two Rust-syntax crates compiled by Ranger (go and es6). The
# same script runs in .github/workflows/test.yml. Exits 1 if anything fails,
# after running everything.
set -u
cd "$(cd "$(dirname "$0")/../../.." && pwd)"
failed=()

run() { # <name> <command...>
  local name="$1"; shift
  local log
  log="$("$@" 2>&1)"
  local rc=$?
  if [[ $rc -ne 0 ]]; then
    failed+=("$name")
    echo "FAIL  $name"
    echo "$log" | tail -40
  else
    echo "ok    $name"
  fi
}

for t in gallery/pptx/tests/*.rgr; do
  n="$(basename "$t" .rgr)"
  run "$n" bash scripts/rgr-suite.sh "./$t" ./gallery/pptx/bin "$n.js"
done

audit() {
  node dist/rgrc.js -es6 ./gallery/pptx/tools/pptx_audit.rgr -d=./gallery/pptx/bin -o=pptx_audit.js -nodecli >/dev/null &&
    node ./gallery/pptx/bin/pptx_audit.js --check
}
run "pptx:audit:check" audit

apijs() {
  bash gallery/pptx/api/js/build.sh >/dev/null && node gallery/pptx/api/js/test.mjs | tee /dev/stderr | grep -q "ALL PASS"
}
run "pptx:api:js:test" apijs

run "rs check go core" bash gallery/pptx/tools/rs_check.sh go core
run "rs check go edit" bash gallery/pptx/tools/rs_check.sh go edit
run "rs check es6 edit" bash gallery/pptx/tools/rs_check.sh es6 edit

if [[ ${#failed[@]} -gt 0 ]]; then
  echo
  echo "${#failed[@]} failed: ${failed[*]}"
  exit 1
fi
echo
echo "all passed"
