#!/usr/bin/env bash
# Check a deployed instance the way a judge would, in under a minute. No tokens needed.
#   scripts/preflight.sh https://your.domain            (web root; the API is expected at /bb)
#   API=http://127.0.0.1:8787 scripts/preflight.sh http://localhost:3000
set -uo pipefail
SITE=${1:?site url}
API=${API:-$SITE/bb}
ok=0; bad=0
check() { # check <label> <command...>
  if "${@:2}" >/dev/null 2>&1; then echo "PASS  $1"; ok=$((ok+1)); else echo "FAIL  $1"; bad=$((bad+1)); fi
}
j() { python3 -I -c "import json,sys; d=json.load(sys.stdin); $1"; }
export -f j

echo "== $SITE (api $API)"
H=$(curl -sf -m 10 "$API/health") || { echo "FAIL  health unreachable"; exit 1; }
echo "$H" | j 'print("   network", d["network"], "tip", d["chain_tip"], "lag", d["sync_lag"], "signing", d.get("signing"), "frozen", d["freeze"]["active"])'
check "health: synced within 3 blocks" bash -c "echo '$H' | j 'assert d[\"sync_lag\"] is not None and d[\"sync_lag\"] <= 3'"
check "health: not parked" bash -c "echo '$H' | j 'assert not d[\"freeze\"][\"active\"]'"
check "rules: deposit menu published" bash -c "curl -sf -m 10 $API/rules | j 'assert len(d[\"deposit_menu_zats\"]) >= 10'"
check "home page renders" bash -c "curl -sf -m 10 $SITE/ | grep -q 'Blind Bazaar'"
check "how page renders" bash -c "curl -sf -m 10 $SITE/how | grep -qi 'how'"
check "embed page renders" bash -c "curl -sf -m 10 $SITE/embed | grep -q 'go-dark'"
check "wasm verifier served" bash -c "curl -sfI -m 10 $SITE/wasm/bb_verify_wasm_bg.wasm | grep -qi 'content-type: application/wasm'"

LOTS=$(curl -sf -m 10 "$API/auctions") || LOTS="[]"
OPEN=$(echo "$LOTS" | j 'print(next((a["id"] for a in d if a["status"] in ("open","sealed")), ""))')
CLOSED=$(echo "$LOTS" | j 'print(next((a["id"] for a in d if a["status"] in ("closed","paid")), ""))')
if [ -n "$OPEN" ]; then
  check "an open lot exists ($OPEN) and its page renders" bash -c "curl -sf -m 10 $SITE/a/$OPEN | grep -q 'Blind Bazaar'"
  check "bid uri for $OPEN builds" bash -c "curl -sf -m 10 '$API/auctions/$OPEN/bid-uri?bid_zats=100000000&refund_address=utest1phxskj2u3phwse5pql5dazp4katjgtx265pnxgputh9p6vgjpc4pt33gmwgmdmqd3gnucf7f073cxlk6c7f6gkzxu3yvgwuysyplaj24' | grep -q '\"uri\":\"zcash:'"
else
  echo "WARN  no open lot: stage one before the demo"
fi
if [ -n "$CLOSED" ]; then
  R=$(curl -sf -m 10 "$API/auctions/$CLOSED/result")
  check "a closed lot ($CLOSED) publishes a ledger root" bash -c "echo '$R' | j 'assert d[\"ledger_hash\"]'"
  check "result page renders" bash -c "curl -sf -m 10 $SITE/a/$CLOSED/result | grep -q 'Blind Bazaar'"
  echo "$R" | j 'print("   clearing", d.get("clearing_zats"), "winners", len(d.get("winning_txids") or []), "disclosure", bool(d.get("disclosure")), "audit", bool(d.get("audit")))'
else
  echo "WARN  no closed lot: the result and proof pages have nothing to show"
fi
check "admin is not reachable from outside" bash -c "[ \$(curl -s -o /dev/null -w '%{http_code}' -m 10 $API/admin/auctions) != 200 ]"
echo "== $ok passed, $bad failed"
[ "$bad" -eq 0 ]
