#!/usr/bin/env bash
# Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
# SPDX-License-Identifier: BSD-3-Clause-Eco
#
# live.sh — five Zoo validators, as processes, each importing Zoo mainnet's
# export, and the anchor that lets them continue it.
#
# An export carries blocks and no certificates, so a validator that read one
# has a tip nothing decided and signs no block on it (the import gate). Each
# signs ONE statement at its tip — the anchor, lux-cpp/node LLM.md — and α of
# the same one lift the gate: the network continues from height 799.
#
#   [1] all five import the export and anchor 799 at Zoo mainnet's tip
#   [2] one seat is down: four anchor; it starts later and takes the anchor its
#       peers hold, off the ledger, and lifts its own gate
#   [3] one seat imported another tip (the export cut at 798): the other four
#       anchor, and it anchors nothing and signs nothing else
#
# Usage: test/live.sh ZOOD CEVM EXPORT     (EXPORT: zoo-mainnet-200200.rlp)
set -uo pipefail

ZOOD="${1:?zood}"
CEVM="${2:?cevm}"
RLP="${3:?zoo-mainnet-200200.rlp}"
BASE="${BASE:-19710}"
RPC="${RPC:-19810}"
N=5
TIP=0xd6f92941bb2ac91dfd2443f32b0d93f2730e6cc81ddb2edd3f35720a9e1f72b8

TMP="$(mktemp -d)"
pids=()
stop_all() {
  for p in "${pids[@]:-}"; do kill -TERM "$p" 2>/dev/null || true; done
  for p in "${pids[@]:-}"; do wait "$p" 2>/dev/null || true; done
  pids=()
}
trap 'stop_all; rm -rf "$TMP"' EXIT

fail=0
check() { if [[ "$2" == 1 ]]; then echo "  ok    $1"; else echo "  FAIL  $1"; fail=1; fi; }
height() {
  curl -s --max-time 5 -X POST -H 'content-type: application/json' \
    --data '{"jsonrpc":"2.0","id":1,"method":"eth_blockNumber","params":[]}' \
    "http://127.0.0.1:$((RPC + $1))/v1/chain/zoo/rpc" | sed -n 's/.*"result":"\([^"]*\)".*/\1/p'
}
# wait_for <log> <pattern> <seconds>
wait_for() {
  for _ in $(seq 1 $(( $3 * 4 ))); do grep -q "$2" "$1" 2>/dev/null && return 0; sleep 0.25; done
  return 1
}

# The same export, cut at height 798: another tip under the same genesis. The
# export opens with block zero, so height 798 is the 799th item.
python3 - "$RLP" "$TMP/cut.rlp" 799 <<'PY'
import sys
data = open(sys.argv[1], 'rb').read()
at, n = 0, 0
while n < int(sys.argv[3]):
    b = data[at]
    if b < 0xf8: raise SystemExit("a block is an RLP list longer than 55 bytes")
    k = b - 0xf7
    at += 1 + k + int.from_bytes(data[at + 1:at + 1 + k], 'big')
    n += 1
open(sys.argv[2], 'wb').write(data[:at])
PY

peers=""
for i in $(seq 0 $((N - 1))); do
  "$ZOOD" --network mainnet --data "$TMP/keys/v$i" --publish >>"$TMP/committee.txt"
  peers="${peers:+$peers,}127.0.0.1:$((BASE + i))"
done

# start <scenario> <seat> <export>
start() {
  local d="$TMP/$1/v$2"
  mkdir -p "$d" && cp -r "$TMP/keys/v$2/." "$d/"
  "$ZOOD" --network mainnet --committee "$TMP/committee.txt" --peers "$peers" --vm "$CEVM" \
          --data "$d" --rpc-port $((RPC + $2)) --deadline-ms 3000 --import-chain-data "$3" \
          >"$TMP/$1/node$2.log" 2>&1 &
  pids+=("$!")
}
anchored() { wait_for "$TMP/$1/node$2.log" "ANCHORED the imported tip at height 799 $TIP" "$3"; }

echo "[1] five seats import Zoo mainnet"
for i in 0 1 2 3 4; do start s1 "$i" "$RLP"; done
all=1; for i in 0 1 2 3 4; do anchored s1 "$i" 90 || all=0; done
check "[1] every seat anchored height 799 at $TIP" "$all"
tips=1; for i in 0 1 2 3 4; do [[ "$(height "$i")" == "0x31f" ]] || tips=0; done
check "[1] every seat serves the imported chain at height 799" "$tips"
lifted=1; for i in 0 1 2 3 4; do grep -q "import gate is lifted" "$TMP/s1/node$i.log" || lifted=0; done
check "[1] and every seat's gate is lifted" "$lifted"
grep -h "ANCHORED" "$TMP"/s1/node*.log | sed 's/^/    /'
stop_all

echo "[2] one seat down, then back"
for i in 0 1 2 4; do start s2 "$i" "$RLP"; done
four=1; for i in 0 1 2 4; do anchored s2 "$i" 90 || four=0; done
check "[2] four of five anchored without seat 3" "$four"
start s2 3 "$RLP"
anchored s2 3 90; check "[2] seat 3 started late and anchored on the certificate its peers hold" "$(( $? == 0 ))"
grep -q "import gate is lifted" "$TMP/s2/node3.log"; check "[2] and lifted its gate" "$(( $? == 0 ))"
stop_all

echo "[3] one seat imported another tip"
for i in 0 1 2 3; do start s3 "$i" "$RLP"; done
start s3 4 "$TMP/cut.rlp"
four=1; for i in 0 1 2 3; do anchored s3 "$i" 90 || four=0; done
check "[3] the four that imported the same tip anchored it" "$four"
sleep 10
! grep -q "ANCHORED" "$TMP/s3/node4.log"; check "[3] the seat that imported 798 blocks anchored nothing" "$(( $? == 0 ))"
[[ "$(height 4)" == "0x31e" ]]; check "[3] and is still at its own tip, 798" "$(( $? == 0 ))"
grep -q "height 798: the tip" "$TMP/s3/node4.log"; check "[3] it offered its own anchor, at 798" "$(( $? == 0 ))"
health="$(curl -s --max-time 5 "http://127.0.0.1:$((RPC + 4))/v1/health")"
echo "$health" | grep -q '"gated":true'; check "[3] and /v1/health says it is gated" "$(( $? == 0 ))"
health="$(curl -s --max-time 5 "http://127.0.0.1:$((RPC + 0))/v1/health")"
echo "$health" | grep -q '"gated":false'; check "[3] while seat 0's says it is not" "$(( $? == 0 ))"
stop_all

echo
if (( fail )); then
  echo "FAIL"
  for f in "$TMP"/s*/node*.log; do echo "--- $f"; tail -12 "$f"; done
  exit 1
fi
echo "PASS — Zoo mainnet imported by five validators, anchored, and one seat cannot stop it"
