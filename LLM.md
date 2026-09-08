# node

`zood` — the Zoo network node. C++20. One binary.

## What this repository is

The Zoo network's identity, as a program. A node is asked which network this
is, what numbers that network is, where its files live, what it binds, and
whether a genesis document is this network's — and this answers those.

The answers are compiled in. A node that read its own identity from somewhere
editable is a node that can be pointed at a fork by editing it.

The Zoo chain is carried by a validator set that also carries other chains, so
the host a validator runs on is shared and the identity is not. This repository
is the identity half. The host half is a C++ package that does not yet install,
export a target, or resolve its own dependencies as packages, so nothing here
depends on it: `zood` binds no mesh and decides no block. What it does state, it
states correctly, and every claim below is a case in `test/check.cpp`.

## Layout

    src/network.h    the three networks, and nothing else about them
    src/network.cpp  finding one by name
    src/genesis.h    what a genesis document says about which network it is for
    src/genesis.cpp  reading one, and what makes one unreadable
    src/main.cpp     the flags, and what is printed
    test/check.cpp   every way a node could end up on the wrong network

Each answers one question. `network` knows nothing about documents; `genesis`
states no identity of its own — it is handed the network to check against.

## Two numbers, not one

A validator joins a NETWORK. A transaction is signed against a CHAIN. They are
different numbers, and collapsing them is how a wallet and a validator come to
disagree about what they are on — visible only after something has been signed
for the wrong thing.

    network id    1 mainnet     2 testnet     3 devnet
    chain id      200200        200201        200202

The chain ids are LP-018's canonical map for this brand. The network ids are the
network whose validator set carries the chain.

Ports are 9630 (RPC) and 9631 (mesh) — what the deployed nodes bind. The data
directory is `~/.zood`.

## What is refused

Refusal is the interesting half. Every one of these has a case in
`test/check.cpp`.

- A network name that is not one of the three. Never a default: a default would
  join the wrong network on a typo and look like it had started correctly.
- A genesis whose chain is not this network's chain.
- A genesis that names a network, when that is not this network. A chain
  document carried inside another network's genesis belongs to that network.
- A field that is present and unreadable — a quoted chain id, a negative one.
  Never a zero.
- A document with no `config`, or a `config` with no chain id.
- An `alloc` that is not a set of accounts.
- A document that wraps itself past the nesting limit, which would otherwise
  recurse until the stack ends.
- `--rpc` and `--mesh` on one address.

## The genesis shapes

Both shapes on disk are read, because both are what is written: the chain's own
document, or a whole-network document carrying it under `cChainGenesis` — as an
object, or as a string holding the document.

## Building

    cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
    cmake --build build -j8
    ctest --test-dir build --output-on-failure

Everything is built `-Wall -Wextra -Wpedantic -Werror`. The JSON reader is one
`FetchContent_Declare` with `FIND_PACKAGE_ARGS`, so the installed package is
used when the host has it and fetched at the pinned tag when it does not — one
declaration, the same build either way.

## Conventions

- Copyright is Zoo Labs Foundation; new files carry `SPDX-License-Identifier:
  BSD-3-Clause-Eco`. Third-party attribution lives in `NOTICE`.
- Another network's brand does not appear in prose, help text, log lines, or
  anything a person reads.
- `LLM.md` is the one document file. `CLAUDE.md`, `AGENTS.md` and `GEMINI.md`
  are gitignored symlinks to it.
