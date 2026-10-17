#!/usr/bin/env bash
# End-to-end money-path test against a local zeckit regtest devnet (zebra + lightwalletd + faucet).
#   zeckit up --backend lwd   (once; Docker must be running)
#   scripts/regtest-e2e.sh
# Exercises: funding, auction creation, two sealed bids from independent wallets, close, refund of the
# loser, winner settlement, seller payout, and the disclosure proof. Prints PASS/FAIL per step.
set -euo pipefail
cd "$(dirname "$0")/.."

BIN=${BIN:-target/debug/bb-auction}
UNITS=${UNITS:-1}   # 1 = single first-price lot; 2 = two-unit uniform-price edition with three bidders
FROST=${FROST:-0}   # 1 = spender under 2-of-3 FROST escrow: two signer hosts co-sign every payout; signer 3 stays offline
FORMAT=${FORMAT:-first_price}   # first_price | vickrey | rfq (UNITS=2 forces uniform). rfq adds a requester wallet that pays the winning quote.
LWD=${LWD:-http://127.0.0.1:9067}
FAUCET=${FAUCET:-http://127.0.0.1:8080}
API=http://127.0.0.1:8797
DATA=$(mktemp -d /tmp/bb-regtest.XXXX)
export NETWORK=regtest REGTEST_NU_HEIGHT=1 REGTEST_LATEST_NU=${REGTEST_LATEST_NU:-nu6_1}
export LIGHTWALLETD_URL=$LWD DATA_DIR=$DATA OPERATOR_SEED_FILE=$DATA/operator.seed BIRTHDAY_HEIGHT=100
export ADMIN_TOKEN=regtest BIND=127.0.0.1:8797 SYNC_INTERVAL_SECS=5 MIN_CONFIRMATIONS=1
export REFUND_DELAY_MIN_SECS=1 REFUND_DELAY_MAX_SECS=2 SETTLEMENT_WINDOW_SECS=600 DISCLOSE_WINNING_MEMO=true RUST_LOG=${RUST_LOG:-info,zcash_client_backend=warn}

pass() { echo "PASS  $*"; }
fail() { echo "FAIL  $*"; echo "--- service log tail ---"; tail -30 "$DATA/service.log"; exit 1; }
j() { python3 -I -c "import json,sys; d=json.load(sys.stdin); print($1)"; }
wait_for() { # wait_for <seconds> <description> <command...>
  local secs=$1 desc=$2; shift 2
  for _ in $(seq 1 "$secs"); do if "$@" >/dev/null 2>&1; then return 0; fi; sleep 1; done
  fail "timeout waiting for: $desc"
}

cargo build -p bb-auction -q
SIGNERS=""
if [ "$FROST" = "1" ]; then
  "$BIN" frost keygen --out "$DATA/frost" > "$DATA/keygen.log" 2>&1 || { cat "$DATA/keygen.log"; exit 1; }
  for i in 1 2; do
    FROST_TOKEN=regtest-frost "$BIN" frost signer --share "$DATA/frost/signer-$i.json" --escrow "$DATA/frost/escrow.json" \
      --bind "127.0.0.1:880$i" --max-external-zats 500000000 > "$DATA/signer-$i.log" 2>&1 &
    SIGNERS="$SIGNERS $!"
  done
  export FROST_ESCROW=$DATA/frost/escrow.json FROST_SIGNERS=http://127.0.0.1:8801,http://127.0.0.1:8802,http://127.0.0.1:8803 FROST_TOKEN=regtest-frost
  wait_for 30 "signer hosts" bash -c "curl -sf http://127.0.0.1:8801/health && curl -sf http://127.0.0.1:8802/health"
  pass "frost keygen done; signers 1 and 2 up, signer 3 deliberately absent"
fi
"$BIN" > "$DATA/service.log" 2>&1 &
SVC=$!
trap 'kill $SVC $SIGNERS 2>/dev/null || true' EXIT

wait_for 60 "service health" bash -c "curl -sf $API/health | grep -q '\"last_sync_at\":[0-9]'"
OP=$(curl -s $API/health | j 'd["operator_address"]')
pass "service synced; operator $OP; signing: $(curl -s $API/health | j 'd["signing"]')"

# Fund operator (fees + refunds) and two bidder wallets from the faucet.
fund() { # fund <address> <zec> ; the faucet syncs between requests, so retry a few times
  for _ in $(seq 1 30); do
    out=$(curl -s --max-time 120 -X POST $FAUCET/send -H 'content-type: application/json' -d "{\"address\":\"$1\",\"amount\":$2}" || true)
    if echo "$out" | grep -q '"txid"'; then echo "  faucet -> ${1:0:16}… $2 ZEC ($(echo "$out" | j 'd["txid"][:12]'))"; return 0; fi
    echo "  faucet retry: ${out:0:120}"; sleep 10
  done
  fail "faucet could not fund $1"
}
fund "$OP" 20.0
pass "faucet funded the operator"
wait_for 240 "operator balance" bash -c "curl -sf $API/health | grep -Eq '\"balance_zats\":[1-9]'"
pass "operator sees funds"

# The operator funds the bidders itself: first exercise of the send path.
BIDDERS="b1 b2"; [ "$UNITS" = "2" ] && BIDDERS="b1 b2 b3"; [ "$FORMAT" = "rfq" ] && BIDDERS="b1 b2 rq"
for b in $BIDDERS; do
  ADDR=$("$BIN" wallet --dir "$DATA/$b" --seed-file "$DATA/$b.seed" 2>/dev/null | awk '/^address:/{print $2}')
  echo "$ADDR" > "$DATA/$b.addr"
  curl -sf -X POST $API/admin/payouts -H 'x-admin-token: regtest' -H 'content-type: application/json' \
    -d "{\"to_address\":\"$ADDR\",\"value_zats\":100000000,\"memo\":\"test funding\"}" >/dev/null || fail "queue funding for $b"
done
NB=$(echo $BIDDERS | wc -w | tr -d ' ')
wait_for 400 "operator funding sends" bash -c "[ \$(curl -sf -H 'x-admin-token: regtest' $API/admin/payouts | python3 -I -c 'import json,sys;print(sum(1 for p in json.load(sys.stdin) if p[\"kind\"]==\"manual\" and p[\"status\"]==\"sent\"))') -ge $NB ]"
pass "operator sent 1 ZEC to each of $NB bidders (shielded sends work)"
if [ "$FROST" = "1" ]; then
  grep -q 'frost session complete' "$DATA/service.log" || fail "no frost session in the service log"
  [ "$(grep -c 'signed .* spend' "$DATA/signer-1.log")" -ge "$NB" ] || fail "signer 1 did not co-sign every funding"
  [ "$(grep -c 'signed .* spend' "$DATA/signer-2.log")" -ge "$NB" ] || fail "signer 2 did not co-sign every funding"
  pass "every send was co-signed by signers 1 and 2 (2-of-3); the spender holds no spending key"
fi

# Auction: reserve 0.2 (RFQ: budget 0.3), deposit 0.1, closes in 8 min (regtest proving + 15 s blocks per bid).
# Seller is the operator itself, except for an RFQ where the requester wallet "rq" creates the lot with a seller key.
SELLER_HDR="x-admin-token: regtest"; SELLER_ADDR=$OP; RESERVE=20000000; FMT_ARG=""
if [ "$UNITS" = "2" ]; then FMT_ARG=""; else FMT_ARG=",\"format\":\"$FORMAT\""; fi
if [ "$FORMAT" = "rfq" ]; then
  SKEY=$(curl -sf -X POST $API/admin/sellers -H 'x-admin-token: regtest' -H 'content-type: application/json' -d '{"name":"requester"}' | j 'd["key"]')
  SELLER_HDR="x-seller-key: $SKEY"; SELLER_ADDR=$(cat "$DATA/rq.addr"); RESERVE=30000000
fi
A=$(curl -sf -X POST $API/auctions -H "$SELLER_HDR" -H 'content-type: application/json' \
  -d "{\"title\":\"regtest lot\",\"reserve_zats\":$RESERVE,\"deposit_zats\":10000000,\"seller_address\":\"$SELLER_ADDR\",\"duration_secs\":480,\"units\":$UNITS$FMT_ARG}")
ID=$(echo "$A" | j 'd["id"]')
pass "auction $ID created (units=$UNITS, format $(echo "$A" | j 'd["format"]'))"

# Sealed bids: b1 0.25, b2 0.3, and for editions b3 0.22. RFQ: b1 quotes 0.25, b2 quotes 0.2 (lowest wins).
BIDS="b1:25000000 b2:30000000"; [ "$UNITS" = "2" ] && BIDS="$BIDS b3:22000000"; [ "$FORMAT" = "rfq" ] && BIDS="b1:25000000 b2:20000000"
for pair in $BIDS; do
  b=${pair%%:*}; amt=${pair##*:}; ADDR=$(cat "$DATA/$b.addr")
  URI=$(curl -sf "$API/auctions/$ID/bid-uri?bid_zats=$amt&refund_address=$ADDR" | j 'd["uri"]')
  "$BIN" wallet --dir "$DATA/$b" --seed-file "$DATA/$b.seed" --wait-balance 10000000 --pay "$URI" 2>/dev/null | grep -E '^txid' || fail "$b could not pay"
  pass "$b paid the deposit with its sealed bid"
done

NBIDS=$(echo $BIDS | wc -w | tr -d ' ')
wait_for 420 "all receipts" bash -c "[ \$(curl -sf $API/auctions/$ID/receipts | python3 -I -c 'import json,sys;print(len(json.load(sys.stdin)[\"receipts\"]))') -ge $NBIDS ]"
pass "all $NBIDS bids ingested from encrypted memos"

wait_for 720 "auction closed" bash -c "curl -sf $API/auctions/$ID/result | grep -q '\"status\":\"closed\"'"
R=$(curl -s $API/auctions/$ID/result)
if [ "$UNITS" = "2" ]; then
  [ "$(echo "$R" | j 'd["clearing_zats"]')" = "25000000" ] || fail "wrong clearing price: $R"
  [ "$(echo "$R" | j 'd["units_sold"]')" = "2" ] || fail "wrong units sold: $R"
  pass "edition: top two win (0.3 and 0.25) at clearing 0.25; 0.22 loses; ledger $(echo "$R" | j 'd["ledger_hash"][:16]')…"
elif [ "$FORMAT" = "vickrey" ]; then
  [ "$(echo "$R" | j 'd["clearing_zats"]')" = "25000000" ] || fail "wrong second price: $R"
  [ "$(echo "$R" | j 'd["price_source"]')" = "second_bid" ] || fail "price not from the second bid: $R"
  [ "$(echo "$R" | j 'd["price_proof"]["proof"]["root"] == d["ledger_hash"]')" = "True" ] || fail "price proof root mismatch: $R"
  pass "vickrey: 0.3 bid wins and pays the 0.25 runner-up; price proof roots to the ledger; winning bid stays sealed"
elif [ "$FORMAT" = "rfq" ]; then
  [ "$(echo "$R" | j 'd["clearing_zats"]')" = "20000000" ] || fail "wrong winning quote: $R"
  pass "rfq: the 0.2 quote beats 0.25 under a 0.3 budget; ledger hash $(echo "$R" | j 'd["ledger_hash"][:16]')…"
else
  [ "$(echo "$R" | j 'd["winning_amount_zats"]')" = "30000000" ] || fail "wrong winner: $R"
  pass "winner is the 0.3 bid; ledger hash $(echo "$R" | j 'd["ledger_hash"][:16]')…"
fi
[ "$(echo "$R" | j 'all(x["verified_against_chain"] for x in d["disclosure"])')" = "True" ] || fail "disclosure not verified: $R"
pass "note-opening disclosure verified against the chain"
if [ "$FORMAT" = "vickrey" ]; then
  [ "$(echo "$R" | j 'all("enc_key" not in x for x in d["disclosure"])')" = "True" ] || fail "vickrey must not disclose the winning memo: $R"
  pass "vickrey: winning memo stays sealed (no key disclosed)"
elif [ "$FORMAT" = "rfq" ]; then
  [ "$(echo "$R" | j 'all(x.get("bid_zats",10**18) <= x.get("reserve_zats",0) for x in d["disclosure"])')" = "True" ] || fail "budget proof missing or over budget: $R"
  pass "winning memo decrypted with the disclosed key; quote is within budget"
else
  [ "$(echo "$R" | j 'all(x.get("bid_zats",0) >= x.get("reserve_zats",10**18) for x in d["disclosure"])')" = "True" ] || fail "reserve proof missing or below reserve: $R"
  pass "winning memo decrypted with the disclosed key; bid meets reserve"
fi

wait_for 420 "loser refunded" bash -c "curl -sf $API/auctions/$ID/result | grep -q '\"kind\":\"refund\",\"status\":\"sent\"'"
pass "loser refund sent"

# Each winner pays price - deposit with the settlement memo BB1S|<auction>|<nonce>.
AADDR=$(echo "$A" | j 'd["address"]')
settle() { # settle <bidder> <zec amount>
  local b=$1 amt=$2
  local WN=$(sqlite3 "$DATA/bb-regtest.sqlite" "SELECT b.nonce FROM bids b WHERE b.auction_id='$ID' AND b.status='won' AND b.refund_address='$(cat "$DATA/$b.addr")'")
  [ -n "$WN" ] || fail "no unpaid winning bid for $b"
  local MEMO="BB1S|$ID|$WN"
  local B64=$(printf '%s' "$MEMO" | base64 | tr '+/' '-_' | tr -d '=')
  "$BIN" wallet --dir "$DATA/$b" --seed-file "$DATA/$b.seed" --wait-balance 1000000 --pay "zcash:$AADDR?amount=$amt&memo=$B64" 2>/dev/null | grep -E '^txid' || fail "$b could not settle"
  pass "$b paid the balance ($amt ZEC)"
}
if [ "$UNITS" = "2" ]; then
  settle b2 0.15   # 0.25 clearing - 0.1 deposit
  settle b1 0.15
elif [ "$FORMAT" = "vickrey" ]; then
  settle b2 0.15   # 0.25 second price - 0.1 deposit
elif [ "$FORMAT" = "rfq" ]; then
  # The requester pays the winning quote with the settlement URI from its dashboard view.
  SURI=$(curl -sf "$API/seller/lots/$ID" -H "$SELLER_HDR" | j 'd["settlement"]["uri"]')
  "$BIN" wallet --dir "$DATA/rq" --seed-file "$DATA/rq.seed" --wait-balance 1000000 --pay "$SURI" 2>/dev/null | grep -E '^txid' || fail "requester could not pay"
  pass "requester paid the 0.2 winning quote"
else
  settle b2 0.2    # 0.3 bid - 0.1 deposit
fi

wait_for 420 "settled" bash -c "curl -sf $API/auctions/$ID/result | grep -Eq '\"status\":\"(settled|paid)\"'"
pass "settlement detected"
wait_for 420 "seller paid" bash -c "curl -sf $API/auctions/$ID/result | grep -q '\"status\":\"paid\"'"
if [ "$FORMAT" = "rfq" ]; then
  PAID=$(sqlite3 "$DATA/bb-regtest.sqlite" "SELECT value_zats FROM payouts WHERE auction_id='$ID' AND kind='seller' AND status='sent'")
  [ "$PAID" = "30000000" ] || fail "quoter should receive quote + bond (0.3), got $PAID"
  pass "winning quoter paid 0.2 quote + 0.1 bond"
else
  pass "seller payout sent"
fi

curl -s $API/auctions/$ID/result | python3 -I -c 'import json,sys; print(json.dumps(json.load(sys.stdin)["disclosure"]))' > "$DATA/disclosure.json"
"$BIN" --verify-disclosure "$DATA/disclosure.json" >/dev/null && pass "CLI verifier: OK" || fail "CLI verifier rejected the disclosure"
curl -s -H 'x-admin-token: regtest' $API/admin/reconcile
if [ "$FROST" = "1" ]; then
  echo "frost sessions: $(grep -c 'frost session complete' "$DATA/service.log") (signer 1: $(curl -s http://127.0.0.1:8801/health | j 'd["signed_sessions"]'), signer 2: $(curl -s http://127.0.0.1:8802/health | j 'd["signed_sessions"]'))"
fi
echo
echo "ALL PASS  (data in $DATA)"
