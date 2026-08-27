# Zoo Chain Node (zood)

Single Go binary that runs the Zoo Network with 3 native VMs:

- **Zoo EVM** — smart contracts, DeFi, AI mining reward distribution
- **Zoo DEX** — CLOB + AMM orderbook matching engine
- **Zoo FHE** — encrypted computation (CKKS/TFHE for private inference, confidential compliance)

## Architecture

```
Zoo Network (L2 on Lux)
  ├── Zoo EVM    — chain ID 200200 (mainnet), 200201 (testnet), 200202 (devnet)
  ├── Zoo DEX    — native orderbook VM (ZOO/USDT, ZOO/ETH pairs)
  └── Zoo FHE    — encrypted computation VM (private ML inference, confidential rebalancing)
```

### Single Binary, Multi-VM

```
zood
  ├── Node mode (default) — full Lux node, self-installs EVM + DEX + FHE plugins
  ├── EVM plugin mode     — Zoo EVM subprocess (LUX_VM_TRANSPORT set)
  ├── DEX plugin mode     — Zoo DEX subprocess
  └── FHE plugin mode     — Zoo FHE subprocess
```

## Chain IDs

| Network | Zoo EVM | Lux Network ID |
|---------|---------|----------------|
| Mainnet | 200200  | 1              |
| Testnet | 200201  | 2              |
| Devnet  | 200202  | 3              |

## Native Token: ZOO

- Fixed supply: 1,000,000,000 (1B ZOO)
- NOT mineable (minted at genesis)
- Used for: governance, staking, gas fees, DEX trading

## $AI Token (via Mining)

- Mineable by offering GPU/CPU compute
- Mining contracts on Zoo EVM
- Teleportable to Lux C-Chain and Hanzo EVM

## Precompiles

All activated at genesis:

| Address | Name | Purpose |
|---------|------|---------|
| 0x0300 | **AI Mining** | Proof of AI — GPU/CPU compute rewards, model hosting, data sharing |
| 0x0700 | **FHE** | Fully Homomorphic Encryption (CKKS/TFHE) — private inference, encrypted compute |
| 0x9010 | LXPool | AMM pool manager (Uniswap V4 style) |
| 0x9020 | LXBook | CLOB order matching |
| 0x9012 | LXRouter | Swap router |
| 0x0200 | ML-DSA | Post-quantum digital signatures |
| 0x0600 | SLH-DSA | Stateless hash-based signatures |
| 0x0800 | FROST | Threshold EdDSA |
| 0x0800+3 | CGGMP21 | Threshold ECDSA |
| 0x020B | Corona | 2-round lattice threshold |
| 0x0500 | Blake3 | Fast hashing |
| 0x0900 | ZK | Zero-knowledge proofs |

## Build

```bash
make build    # produces ./zood
make test     # run tests
```

**Keep the Dockerfile's Go builder patch-pinned (`golang:1.26.5-bookworm`) and keep
`ENV GOTOOLCHAIN=auto`.** The official `golang` images ship `GOTOOLCHAIN=local`, so
a floating `golang:1.26` tag that lands one patch behind the `go` directive in
go.mod fails the build outright (`go.mod requires go >= X (running go Y;
GOTOOLCHAIN=local)`). The pin makes the build hermetic; `GOTOOLCHAIN=auto` means a
future go.mod bump downloads its toolchain rather than hard-failing. Do not unpin.

A plain `go build ./...` will NOT work here — `github.com/luxfi/precompile@v0.5.38`
mismatches go.sum because luxfi tags get rewritten. That is why the Dockerfile does
`sed -i '/luxfi\//d' go.sum` before `go mod download`. To reproduce the container
build locally, strip those go.sum lines the same way and build with
`GOFLAGS=-mod=mod GOSUMDB=off GOEXPERIMENT=jsonv2` — do not commit the stripped
go.sum.

## L3 App-Chains

Zoo supports L3 app-chains deployed on top:

```bash
zoo chain create beluga --type=l3 --evm-chain-id=420420 --token-name=BELUGA --token-symbol=BLG
zoo chain deploy beluga --local
```

## Related Repos

| Repo | Purpose |
|------|---------|
| zoo-labs/cli | CLI (zoo network start, zoo chain create/deploy) |
| zoo-labs/node | Rust AI mining node (agents, inference, P2P) |
| zooai/operator | K8s operator for Zoo network |
| zooai/universe | Deployment manifests |

## Convergence: this repo vs `hanzoai/node`

This repo (`zooai/node`) and `hanzoai/node` share the same Rust node source
(`hanzo-bin/hanzo-node/src` is ~identical between the two), but they play
**different roles** in the shared build:

- **`zooai/node` (this repo) = publish origin / fat monorepo.** It vendors all
  40+ `hanzo-*` node libraries under `hanzo-libs/*` as path crates. These are the
  *source of truth* that gets published to crates.io (see `publish-crates.sh`,
  `prepare-publish.sh`, `rename-to-idiomatic.sh`). It also carries the Go chain
  layer (`zood`, `bootstrap*.go`, …) and the decomposed mining/L2 crates
  (`hanzo-mining`, `hanzo-consensus`, `hanzo-compute`, `hanzo-l2`).
- **`hanzoai/node` = slim consumer.** Its workspace has only 4 members
  (`hanzo-test-framework`, `hanzo-test-macro`, `hanzo-bin/hanzo-node`,
  `hanzo-bin/hanzoai`) and pulls every `hanzo-*` lib **from crates.io** at the
  unified version line, plus `hanzo-engine` (path `../engine/hanzo-engine`).

These are **separate git histories** (no common ancestor). They are NOT merged
with `git merge`/`git pull`. Cross-repo transfer is a *feature port* driven by an
explicit token rename map (`hanzo`↔`zoo`), copy → fix imports → build → commit.

### `zoo-libs/` is legacy
`zoo-libs/*`, `zoo-bin/zoo-node`, `zoo-test-framework`, `zoo-test-macro` are the
pre-rename Shinkai-lineage originals. They are **not** workspace members and no
`hanzo-*` crate depends on them. They are retained for reference and slated for
removal once the rename is finalized (tracked for human review).

### Release order
`ml → engine → node` (the leaf is `hanzoai/ml`; `hanzoai/engine` depends on `ml`;
both nodes depend on `engine`). Publish `hanzo-*` node libs from **this** repo at
a single version line; `hanzoai/node` then consumes them from crates.io.
