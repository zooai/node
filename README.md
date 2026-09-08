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

```
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j8
```

The binary is `build/zood`.

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
