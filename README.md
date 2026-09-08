# node

`zood` — the Zoo network node.

It resolves the network it runs and checks a genesis document against that
network.

## Networks

| network | number |
| ------- | ------ |
| mainnet | 200200 |
| testnet | 200201 |
| devnet  | 200202 |

One number per network: it is the network id peers greet each other with and
the EVM chain id a transaction is signed against. The three are compiled in. A
name that is not one of them is an error, never a default.

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
id         200201
data       ./n0
rpc        127.0.0.1:9630
mesh       127.0.0.1:9631
genesis    genesis.json
states     200201
allocates  3 accounts
```

## What is refused

A genesis is checked, not trusted. It must state the number of the network it
was named with:

```
$ zood --network mainnet --genesis genesis.json
zood: genesis.json: this genesis is network 200201, not mainnet (200200)
```

A whole-network document states that number twice, and both must be the same
number — a document whose two halves name different networks is one half of two
genesis files, and whichever the node believed would be wrong somewhere else:

```
$ zood --network mainnet --genesis split.json
zood: split.json: it is network 200200 carrying chain 200201
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
