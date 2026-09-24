# Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
# SPDX-License-Identifier: BSD-3-Clause-Eco
#
# A Zoo network's history, asked for through the door a client uses.
#
#   python3 accept.py NETWORK HOST:PORT [--alone] [VALIDATOR_LOG...]
#
# Waits for the node to reach the export's tip, then checks what luxfi/state's
# export and the Go archive say: the block hashes, the transactions, the chain
# id, and that the chain is Zoo's and not the C-Chain.

import json
import sys
import time
import urllib.error
import urllib.request

# What each network's export and its Go archive say.
NETWORKS = {
    "mainnet": {
        "number": "200200", "chain": "0x30e08", "tip": 799, "txs": 811,
        "blocks": {
            0: "0x7c548af47de27560779ccc67dda32a540944accc71dac3343da3b9cd18f14933",
            400: "0xa8f981809d9f9f5d3f4e381b68af76f384a5d023a1b8f3f7b52726c14f242585",
            799: "0xd6f92941bb2ac91dfd2443f32b0d93f2730e6cc81ddb2edd3f35720a9e1f72b8",
        },
    },
    "testnet": {
        "number": "200201", "chain": "0x30e09", "tip": 84, "txs": 84,
        "blocks": {
            0: "0x0652fb2fde1460544a5893e5eba5095ff566861cbc87fcb1c73be2b81d6d1979",
            42: "0x8aa48eac58059ddd70133c507fbe2c9573ece034f255cba087394a78706cdf81",
            84: "0x7077c4e52ace2a56dd1189b2aaf91ba9ec338d86c5b4530b0ac98a8ff6d8ba0c",
        },
    },
}

args = sys.argv[1:]
NET = NETWORKS[args.pop(0)]
HOST = args.pop(0)
ALONE = "--alone" in args  # a committee of one: it must refuse transactions
LOGS = [a for a in args if a != "--alone"]
TIP = NET["tip"]
WANT = NET["blocks"]
TXS = NET["txs"]

failed = 0


def check(ok, what):
    global failed
    print(("  ok    " if ok else "  FAIL  ") + what, flush=True)
    if not ok:
        failed += 1


def call(path, method, params):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
    req = urllib.request.Request(f"http://{HOST}{path}", data=body,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status, json.load(r)
    except urllib.error.HTTPError as e:
        return e.code, None


rpc = "/v1/chain/zoo/rpc"
deadline = time.time() + 600
while True:
    try:
        status, body = call(rpc, "eth_blockNumber", [])
        if status == 200 and body.get("result") == hex(TIP):
            break
    except OSError:
        pass
    for log in LOGS:
        with open(log, errors="replace") as f:
            if "cannot start" in f.read():
                sys.exit(f"a validator did not start: {log}")
    if time.time() > deadline:
        sys.exit(f"zood did not reach block {TIP}")
    time.sleep(2)

print(f"zood at block {TIP} on {HOST}")
status, body = call(rpc, "eth_chainId", [])
check(status == 200 and body.get("result") == NET["chain"], f"eth_chainId is {NET['chain']}")
for n, h in WANT.items():
    status, body = call(rpc, "eth_getBlockByNumber", [hex(n), False])
    got = (body or {}).get("result") or {}
    check(got.get("hash") == h, f"block {n} is {h[:12]}… (got {str(got.get('hash'))[:12]}…)")

txs = 0
for n in range(1, TIP + 1):
    _, body = call(rpc, "eth_getBlockByNumber", [hex(n), False])
    txs += len(((body or {}).get("result") or {}).get("transactions") or [])
check(txs == TXS, f"blocks 1..{TIP} carry {TXS} transactions (got {txs})")

status, body = call(f"/v1/chain/{NET['number']}/rpc", "eth_chainId", [])
check(status == 200 and body.get("result") == NET["chain"],
      f"/v1/chain/{NET['number']} is the same chain")
status, _ = call("/v1/chain/c/rpc", "eth_chainId", [])
check(status == 404, f"/v1/chain/c is not Zoo's (got {status})")
_, body = call(rpc, "admin_importChain", ["/nonexistent"])
check(((body or {}).get("error") or {}).get("code") == -32601, "admin_* is off")

if ALONE:
    # A committee of one certifies nothing, so it takes nothing it cannot land.
    _, body = call(rpc, "eth_sendRawTransaction", ["0x00"])
    msg = (((body or {}).get("error") or {}).get("message") or "")
    check("accepts no transactions" in msg, "a transaction is refused, with the reason")

print("PASS" if failed == 0 else "FAIL", flush=True)
sys.exit(1 if failed else 0)
