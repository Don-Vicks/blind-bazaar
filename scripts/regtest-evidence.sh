#!/usr/bin/env bash
# Turn a regtest run log into an evidence file: the PASS/FAIL lines and the faucet funding lines, nothing typed by hand.
#   scripts/regtest-evidence.sh <name> <log>   -> docs/evidence/regtest/<name>.md
set -euo pipefail
name=$1; log=$2; out="docs/evidence/regtest/$name.md"
{
  echo "# regtest run: $name ($(date -u +%Y-%m-%d))"
  echo
  echo "Local zeckit devnet (zebra + lightwalletd + faucet, 15 s blocks), \`scripts/regtest-e2e.sh\` with \`$(grep -m1 -oE '(UNITS|FORMAT|FROST)=[a-z0-9_]+' <<<"$name" || echo "defaults")\`. Every line below is the script's own output."
  echo
  echo '```'
  grep -E '^(PASS|FAIL|  faucet)' "$log" | cut -c1-200
  echo '```'
  echo
  echo "Result: $(grep -cE '^PASS' "$log") passed, $(grep -cE '^FAIL' "$log") failed."
} > "$out"
echo "wrote $out"
