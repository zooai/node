# node

`zood` — the Zoo network's node. lux-cpp/node's `run()` on Zoo's three specs
(`src/zood.cpp`), with cevm as the EVM plugin its chain runs in. C++20. One
binary, and the plugin beside it.

## What this repository is

The Zoo network's identity: its three networks, their numbers, their genesis
documents (compiled in: a node that read its identity from somewhere editable
can be pointed at a fork by editing it), and its acceptance tests. Everything
else — the mesh, the height rule, the anchor, the journal, the RPC — is
lux-cpp/node's, pinned by commit in the Dockerfile, so every Zoo validator runs
the one implementation its peers must match.

    src/zood.cpp        the specs and main
    src/genesis.*       the three genesis documents, compiled in
    genesis/*.json      the documents
    test/accept.py      a network's history, through the door a client uses
    test/live.sh        five validators import mainnet and anchor its tip

## Two numbers, not one

    network id    1 mainnet     2 testnet     3 devnet
    chain id      200200        200201        200202

A validator joins a NETWORK (the one whose validator set carries Zoo); a transaction
is signed against a CHAIN. Ports: 9630 (RPC), 9631 (mesh).

## Continuing Zoo's history

luxfi/state's exports carry blocks and no certificates, so a validator that
imports one holds a tip no quorum decided and signs no block on it (the import
gate, lux-cpp/node LLM.md). Each validator signs one anchor on its own tip —
mainnet: height 799, `0xd6f92941bb2ac91dfd2443f32b0d93f2730e6cc81ddb2edd3f35720a9e1f72b8`
— and four of five on the same tip are the certificate that lifts every gate;
800 is built on it. A validator that was down asks its peers when it returns and
lifts its gate on the anchor they hold; one that imported another tip never
reaches four and stays out.

`test/live.sh ZOOD CEVM EXPORT` (Dockerfile `--target accept` runs it):

- [1] five processes import `zoo-mainnet-200200.rlp`; all five anchor 799 at the
  tip above, and serve the chain at 799
- [2] seat 3 down: four anchor; seat 3 starts later and anchors on its peers'
  ledger
- [3] seat 4 imports the export cut at 798: the other four anchor, it anchors
  nothing and stays at 798

Measured on the build pod against lux-cpp/node fix/live: all pass. Building
800 over cevm needs a transaction from a funded Zoo account; the building is
lux-cpp/node's `live_test` (the same tip, a stand-in chain) and `cluster.sh`
(processes over cevm, the local chain).

## Building

    cmake -S . -B build -DLUX_NODE_DIR=<lux-cpp/node> <the node's -D flags>
    cmake --build build --target zood cevm-bin

The image is the Dockerfile alone: every source a full commit SHA in an `ARG`,
checked against the branch or tag it is pinned from.

## Conventions

- Copyright is Zoo Labs Foundation; new files carry `SPDX-License-Identifier:
  BSD-3-Clause-Eco`. Third-party attribution lives in `NOTICE`.
- Another network's brand does not appear in prose, help text, log lines, or
  anything a person reads.
- `LLM.md` is the one document file. `CLAUDE.md`, `AGENTS.md` and `GEMINI.md`
  are gitignored symlinks to it.
