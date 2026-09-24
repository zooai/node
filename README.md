# node

`zood` — the Zoo network node: lux-cpp/node run on Zoo's specs
([src/zood.cpp](src/zood.cpp)), with cevm as the EVM plugin its chain runs in.

## Networks

| network | network id | chain id | genesis |
| ------- | ---------- | -------- | ------- |
| mainnet | 1          | 200200   | [genesis/mainnet.json](genesis/mainnet.json), block 0 `0x7c548af4…` |
| testnet | 2          | 200201   | [genesis/testnet.json](genesis/testnet.json), block 0 `0x0652fb2f…` |
| devnet  | 3          | 200202   | [genesis/devnet.json](genesis/devnet.json), a new chain |

Zoo is an L2 on Lux's validator set, so its networks greet under Lux's network
ids; the chains are its own. mainnet and testnet start from the genesis their
history does (luxfi/state `chains/zoo-{mainnet,testnet}`), so luxfi/state's
exports import onto them. The documents are compiled in, and the node refuses a
genesis that names another chain.

## Run

```
zood [--network NAME] --data DIR --publish
zood [--network NAME] --committee FILE --peers a:p,b:p,... [--data DIR]
     [--rpc-host H] [--rpc-port R] [--import-chain-data PATH] [--vm PATH]
```

`--publish` prints this validator's committee line; `--committee` names every
validator of the network, and `--peers` their mesh addresses in the same order.
In a pod, `--rpc-host 0.0.0.0 --rpc-port 9630 --data /data`. The chain's RPC is
`/v1/chain/zoo/rpc` (and `/v1/chain/200200/rpc`); `/v1/chain/c` is a 404.

## Build

The image, `ghcr.io/zooai/node`, is compiled from source by the
[Dockerfile](Dockerfile) alone. Every source repository is a full commit SHA in
an `ARG` at its top, checked to be on the branch or tag it is pinned from.
Private repositories are read with the BuildKit secret `GH_READ_TOKEN`; only the
two steps that fetch see it.

```
hanzo platform runner --as zoo --repo https://github.com/zooai/node \
  --sha <commit> --dockerfile Dockerfile --image ghcr.io/zooai/node:<commit>
```

The binaries are `/usr/local/bin/zood` and `/usr/local/libexec/lux/cevm`.

## License

Zoo Ecosystem License — see [LICENSE](LICENSE). Third-party components are
listed in [NOTICE](NOTICE).
