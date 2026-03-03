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
