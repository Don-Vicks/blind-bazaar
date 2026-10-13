#!/usr/bin/env bash
# End-to-end money-path test against a local zeckit regtest devnet (zebra + lightwalletd + faucet).
#   zeckit up --backend lwd   (once; Docker must be running)
#   scripts/regtest-e2e.sh
# Exercises: funding, auction creation, two sealed bids from independent wallets, close, refund of the
# loser, winner settlement, seller payout, and the disclosure proof. Prints PASS/FAIL per step.
set -euo pipefail
cd "$(dirname "$0")/.."

BIN=${BIN:-target/debug/bb-auction}
LWD=${LWD:-http://127.0.0.1:9067}
FAUCET=${FAUCET:-http://127.0.0.1:8080}
API=http://127.0.0.1:8797
DATA=$(mktemp -d /tmp/bb-regtest.XXXX)
export NETWORK=regtest REGTEST_NU_HEIGHT=1 REGTEST_LATEST_NU=${REGTEST_LATEST_NU:-nu6_1}
export LIGHTWALLETD_URL=$LWD DATA_DIR=$DATA OPERATOR_SEED_FILE=$DATA/operator.seed BIRTHDAY_HEIGHT=1
export ADMIN_TOKEN=regtest BIND=127.0.0.1:8797 SYNC_INTERVAL_SECS=5 MIN_CONFIRMATIONS=1
export REFUND_DELAY_MIN_SECS=1 REFUND_DELAY_MAX_SECS=2 SETTLEMENT_WINDOW_SECS=600 RUST_LOG=${RUST_LOG:-info,zcash_client_backend=warn}

pass() { echo "PASS  $*"; }
fail() { echo "FAIL  $*"; echo "--- service log tail ---"; tail -30 "$DATA/service.log"; exit 1; }
j() { python3 -I -c "import json,sys; d=json.load(sys.stdin); print($1)"; }
wait_for() { # wait_for <seconds> <description> <command...>
  local secs=$1 desc=$2; shift 2
  for _ in $(seq 1 "$secs"); do if "$@" >/dev/null 2>&1; then return 0; fi; sleep 1; done
  fail "timeout waiting for: $desc"
}

cargo build -p bb-auction -q
"$BIN" > "$DATA/service.log" 2>&1 &
SVC=$!
trap 'kill $SVC 2>/dev/null || true' EXIT

wait_for 60 "service health" bash -c "curl -sf $API/health | grep -q '\"last_sync_at\":[0-9]'"
OP=$(curl -s $API/health | j 'd["operator_address"]')
pass "service synced; operator $OP"

# Fund operator (fees + refunds) and two bidder wallets from the faucet.
fund() { curl -sf -X POST $FAUCET/send -H 'content-type: application/json' -d "{\"address\":\"$1\",\"amount\":$2}" >/dev/null; }
fund "$OP" 1.0
for b in b1 b2; do
  ADDR=$("$BIN" wallet --dir "$DATA/$b" --seed-file "$DATA/$b.seed" 2>/dev/null | awk '/^address:/{print $2}')
  echo "$ADDR" > "$DATA/$b.addr"; fund "$ADDR" 0.5
done
pass "faucet funded operator and bidders"

wait_for 180 "operator balance" bash -c "curl -sf $API/health | grep -Eq '\"balance_zats\":[1-9]'"
pass "operator sees funds"

# Auction: reserve 0.2, deposit 0.1, closes in 90 s. Seller is the operator itself for the test.
A=$(curl -sf -X POST $API/auctions -H 'x-admin-token: regtest' -H 'content-type: application/json' \
  -d "{\"title\":\"regtest lot\",\"reserve_zats\":20000000,\"deposit_zats\":10000000,\"seller_address\":\"$OP\",\"duration_secs\":90}")
ID=$(echo "$A" | j 'd["id"]')
pass "auction $ID created"

# Two sealed bids: b1 bids 0.25, b2 bids 0.3 (winner). Each bidder refunds to its own address.
for pair in "b1:25000000" "b2:30000000"; do
  b=${pair%%:*}; amt=${pair##*:}; ADDR=$(cat "$DATA/$b.addr")
  URI=$(curl -sf "$API/auctions/$ID/bid-uri?bid_zats=$amt&refund_address=$ADDR" | j 'd["uri"]')
  "$BIN" wallet --dir "$DATA/$b" --seed-file "$DATA/$b.seed" --wait-balance 10000000 --pay "$URI" 2>/dev/null | grep -E '^txid' || fail "$b could not pay"
  pass "$b paid the deposit with its sealed bid"
done

wait_for 180 "two receipts" bash -c "[ \$(curl -sf $API/auctions/$ID/receipts | python3 -I -c 'import json,sys;print(len(json.load(sys.stdin)[\"receipts\"]))') -ge 2 ]"
pass "both bids ingested from encrypted memos"

wait_for 240 "auction closed" bash -c "curl -sf $API/auctions/$ID/result | grep -q '\"status\":\"closed\"'"
R=$(curl -s $API/auctions/$ID/result)
[ "$(echo "$R" | j 'd["winning_amount_zats"]')" = "30000000" ] || fail "wrong winner: $R"
pass "winner is the 0.3 bid; ledger hash $(echo "$R" | j 'd["ledger_hash"][:16]')…"
[ "$(echo "$R" | j 'd["disclosure"]["verified_against_chain"]')" = "True" ] || fail "disclosure not verified: $R"
pass "note-opening disclosure verified against the chain"

wait_for 240 "loser refunded" bash -c "curl -sf $API/auctions/$ID/result | grep -q '\"kind\":\"refund\",\"status\":\"sent\"'"
pass "loser refund sent"

# Winner (b2) pays the balance 0.3 - 0.1 = 0.2 with the settlement memo.
# The settlement URI needs the winner's nonce; fetch it from the bid record through the admin DB.
WN=$(sqlite3 "$DATA/bb-regtest.sqlite" "SELECT b.nonce FROM auctions a JOIN bids b ON b.id=a.winner_bid_id WHERE a.id='$ID'")
AADDR=$(echo "$A" | j 'd["address"]')
MEMO="BB1S|$ID|$WN"
B64=$(printf '%s' "$MEMO" | base64 | tr '+/' '-_' | tr -d '=')
SURI="zcash:$AADDR?amount=0.2&memo=$B64"
"$BIN" wallet --dir "$DATA/b2" --seed-file "$DATA/b2.seed" --wait-balance 20000000 --pay "$SURI" 2>/dev/null | grep -E '^txid' || fail "winner could not settle"
pass "winner paid the balance"

wait_for 240 "settled" bash -c "curl -sf $API/auctions/$ID/result | grep -Eq '\"status\":\"(settled|paid)\"'"
pass "settlement detected"
wait_for 240 "seller paid" bash -c "curl -sf $API/auctions/$ID/result | grep -q '\"status\":\"paid\"'"
pass "seller payout sent"

curl -s $API/auctions/$ID/result | j 'd["disclosure"]' > "$DATA/disclosure.json"
"$BIN" --verify-disclosure "$DATA/disclosure.json" >/dev/null && pass "CLI verifier: OK" || fail "CLI verifier rejected the disclosure"
curl -s -H 'x-admin-token: regtest' $API/admin/reconcile
echo
echo "ALL PASS  (data in $DATA)"
