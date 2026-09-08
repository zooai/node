// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// SPDX-License-Identifier: BSD-3-Clause-Eco
//
// zood — the Zoo network node.
//
// It answers the questions a node is asked before it can do anything: which
// network this is, what number that network is and what number the chain on it
// is, where its files live, and what it binds. Given a genesis document it also answers whether that document
// is this network's — because a node that started on someone else's genesis
// would produce blocks nobody else accepts, and would look like it had started
// correctly.
//
// EVERY ANSWER IS COMPILED IN OR REFUSED. The networks are three constants, not
// a file: a node that read its own identity from somewhere editable is a node
// that can be pointed at a fork by editing it. An unknown network name is an
// error and never a default, because a default would join the wrong network on
// a typo.

#include "genesis.h"
#include "network.h"

#include <cstdlib>
#include <exception>
#include <cstddef>
#include <filesystem>
#include <iostream>
#include <string>
#include <string_view>
#include <vector>

namespace {

constexpr std::string_view kVersion = "0.1.0";

constexpr std::string_view kUsage = R"(zood — the Zoo network node

  --network <name>   mainnet | testnet | devnet        (default mainnet)
  --data <dir>       where this node's files live      (default ~/.zood)
  --rpc <addr>       JSON-RPC address                  (default 127.0.0.1:9630)
  --mesh <addr>      validator mesh address            (default 127.0.0.1:9631)
  --genesis <file>   a genesis document to check against this network
  --version          print the version, and exit
  --help             print this text, and exit

A genesis is checked, not trusted: the document must state the number of the
network it was named with. A genesis that disagrees is refused here, where the
refusal is one line, rather than at the first block nobody else accepts.
)";

// The value after `flag`, if it is there.
std::optional<std::string> flag(const std::vector<std::string>& args, std::string_view name) {
    for (std::size_t i = 0; i + 1 < args.size(); ++i)
        if (args[i] == name) return args[i + 1];
    return std::nullopt;
}

bool given(const std::vector<std::string>& args, std::string_view name) {
    for (const auto& a : args)
        if (a == name) return true;
    return false;
}

// `1 account`, `3 accounts`.
std::string plural(std::size_t n, std::string_view thing) {
    return std::to_string(n) + " " + std::string(thing) + (n == 1 ? "" : "s");
}

// Where a person's files live.
std::string home() {
    if (const char* h = std::getenv("HOME"); h && *h) return h;
    return {};
}

}  // namespace

int main(int argc, char** argv) {
    const std::vector<std::string> args(argv + 1, argv + argc);

    if (given(args, "--help") || given(args, "-h")) {
        std::cout << kUsage;
        return 0;
    }
    if (given(args, "--version")) {
        std::cout << "zood " << kVersion << "\n";
        return 0;
    }

    const std::string name = flag(args, "--network").value_or("mainnet");
    const auto net = zoo::network_named(name);
    if (!net) {
        std::cerr << "zood: " << name << " is not a network; try " << zoo::network_names() << "\n";
        return 1;
    }

    std::string data = flag(args, "--data").value_or("");
    if (data.empty()) {
        const std::string h = home();
        if (h.empty()) {
            std::cerr << "zood: HOME is not set, so --data has no default; pass it\n";
            return 1;
        }
        data = (std::filesystem::path(h) / ".zood").string();
    }

    const std::string rpc = flag(args, "--rpc").value_or("127.0.0.1:9630");
    const std::string mesh = flag(args, "--mesh").value_or("127.0.0.1:9631");
    if (rpc == mesh) {
        std::cerr << "zood: --rpc and --mesh cannot be one address\n";
        return 1;
    }

    std::cout << "zood " << kVersion << "\n"
              << "network    " << net->name << "\n"
              << "network id " << net->id << "\n"
              << "chain id   " << net->chain << "\n"
              << "data       " << data << "\n"
              << "rpc        " << rpc << "\n"
              << "mesh       " << mesh << "\n";

    if (const auto path = flag(args, "--genesis")) {
        try {
            const zoo::Genesis g = zoo::read_genesis(*path);
            g.agrees_with(*net);
            std::cout << "genesis    " << *path << "\n"
                      << "states     " << g.chain << "\n"
                      << "allocates  " << plural(g.accounts, "account") << "\n";
        } catch (const std::exception& e) {
            std::cerr << "zood: " << *path << ": " << e.what() << "\n";
            return 1;
        }
    }
    return 0;
}
