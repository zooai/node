# node

`zood` — the Zoo network node.

It resolves the network it runs, and checks a genesis document against that
network.

## Networks

| network | network id | chain id |
| ------- | ---------- | -------- |
| mainnet | 1          | 200200   |
| testnet | 2          | 200201   |
| devnet  | 3          | 200202   |

Two numbers, because there are two questions. The network id is what validators
greet each other under. The chain id is what a transaction is signed against.
Collapsing them into one number is how a wallet and a validator come to disagree
about what they are on, and the disagreement is only visible after a transaction
has been signed for the wrong thing.

The three are compiled in. A name that is not one of them is an error, never a
default.

## Build

The image, `ghcr.io/zooai/node`, is compiled from source by the
[Dockerfile](Dockerfile) alone: luxfi/crypto's post-quantum archive, AWS-LC,
cevm's Conan dependencies, the consensus engine and lux-cpp/node, then `zood`
into `gcr.io/distroless/cc-debian12:nonroot`. Every source repository is a full
commit SHA in an `ARG` at the top of that file, and each is checked to be on the
branch or tag it is pinned from. Debian's packages, Conan's Python dependencies
and ConanCenter's packages still resolve when the image is built.

Some of those repositories are private. The build reads them with the BuildKit
secret `GH_READ_TOKEN`, a GitHub token that can read lux-cpp, lux-gpu and luxfi,
and stops with that name when it is missing. Only the two steps that fetch see
it, and they run git alone. It is never an `ARG` or a file, and each of those
steps fails if it finds the token in what it leaves.

On the Hanzo platform, both architectures, each built natively, under one tag.
The build runs as the zoo org: that is what may push `ghcr.io/zooai/*`, and
what the platform hands `GH_READ_TOKEN` to.

```
hanzo platform runner --as zoo --repo https://github.com/zooai/node \
  --sha <commit> --dockerfile Dockerfile --image ghcr.io/zooai/node:<commit>
```

The binary is `/usr/local/bin/zood`. The Dockerfile's last `cmake` step is the
local build, given checkouts of the same commits.

## Run

```
zood --network testnet --data ./n0 --genesis genesis.json
```

```
zood 0.1.0
network    testnet
network id 2
chain id   200201
data       ./n0
rpc        127.0.0.1:9630
mesh       127.0.0.1:9631
genesis    genesis.json
states     200201
allocates  2 accounts
```

## What is refused

A genesis is checked, not trusted. Its chain must be this network's chain:

```
$ zood --network mainnet --data ./n0 --genesis genesis.json
zood: genesis.json: this genesis is chain 200201, not mainnet's chain 200200
```

And when a document names a network, it must name this one — a chain document
carried inside another network's genesis belongs to that network:

```
$ zood --network testnet --data ./n0 --genesis deployed.json
zood: deployed.json: this genesis is network 200201, not testnet (2)
```

A name that is not a network is an error rather than a default, because a
default would join the wrong network on a typo and look like it had started
correctly:

```
$ zood --network mainet
zood: mainet is not a network; try mainnet | testnet | devnet
```

## Flags

```
  --network <name>   mainnet | testnet | devnet        (default mainnet)
  --data <dir>       where this node's files live      (default ~/.zood)
  --rpc <addr>       JSON-RPC address                  (default 127.0.0.1:9630)
  --mesh <addr>      validator mesh address            (default 127.0.0.1:9631)
  --genesis <file>   a genesis document to check against this network
  --version          print the version, and exit
  --help             print this text, and exit
```

## Test

```
ctest --test-dir build --output-on-failure
```

## License

Zoo Ecosystem License — see [LICENSE](LICENSE). Third-party components are
listed in [NOTICE](NOTICE).
