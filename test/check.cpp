// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// SPDX-License-Identifier: BSD-3-Clause-Eco
//
// What this node must get right about its own identity and about the documents
// it is handed. Every case here is a way a node could end up on the wrong
// network while appearing to have started correctly.

#include "genesis.h"
#include "network.h"

#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>

#include <unistd.h>

namespace {

int failures = 0;

void ok(bool held, const std::string& what) {
    if (held) {
        std::cout << "ok   " << what << "\n";
        return;
    }
    std::cout << "FAIL " << what << "\n";
    ++failures;
}

// A genesis file holding `body`, removed when the run ends.
std::string wrote(const std::string& tag, const std::string& body) {
    const auto path = std::filesystem::temp_directory_path() /
                      ("zood-" + std::to_string(::getpid()) + "-" + tag + ".json");
    std::ofstream(path) << body;
    return path.string();
}

// Whether reading `path` is refused.
bool refused(const std::string& path) {
    try {
        zoo::read_genesis(path);
        return false;
    } catch (const std::exception&) {
        return true;
    }
}

// Whether reading `path` yields a genesis this network accepts.
bool accepted(const std::string& path, const zoo::Network& net) {
    try {
        zoo::read_genesis(path).agrees_with(net);
        return true;
    } catch (const std::exception&) {
        return false;
    }
}

}  // namespace

int main() {
    // ── the networks ────────────────────────────────────────────────────────
    for (const zoo::Network& n : zoo::kAll) {
        const auto found = zoo::network_named(n.name);
        ok(found && found->id == n.id && found->chain == n.chain,
           std::string(n.name) + " is reachable by its name");
    }
    ok(!zoo::network_named("mainet"), "a typo is not a network");
    ok(!zoo::network_named(""), "no name is not a network");
    ok(!zoo::network_named("MAINNET"), "another spelling is not a network");
    for (std::size_t i = 0; i < std::size(zoo::kAll); ++i)
        for (std::size_t j = i + 1; j < std::size(zoo::kAll); ++j) {
            ok(zoo::kAll[i].id != zoo::kAll[j].id, "no two networks share a network id");
            ok(zoo::kAll[i].chain != zoo::kAll[j].chain, "no two networks share a chain id");
        }
    // The mistake this table exists to prevent: one number doing both jobs.
    for (const zoo::Network& n : zoo::kAll)
        ok(n.id != n.chain, std::string(n.name) + " does not name itself twice");

    // ── a genesis that is this network's ────────────────────────────────────
    const std::string mine = wrote("mine", R"({"config":{"chainId":200200},
        "alloc":{"0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266":{"balance":"0x1"}}})");
    ok(accepted(mine, zoo::kMainnet), "a genesis stating this network is accepted");
    ok(zoo::read_genesis(mine).accounts == 1, "and its allocation is counted");

    // ── a genesis that is not ───────────────────────────────────────────────
    ok(!accepted(mine, zoo::kTestnet), "a genesis for another network is refused");
    const std::string theirs = wrote("theirs", R"({"config":{"chainId":96369}})");
    ok(!accepted(theirs, zoo::kMainnet), "and so is one for another network entirely");

    // ── the shapes a genesis is written in ──────────────────────────────────
    const std::string wrapped = wrote("wrapped",
        R"({"networkID":2,"cChainGenesis":{"config":{"chainId":200201}}})");
    ok(accepted(wrapped, zoo::kTestnet), "a whole-network document is read");
    const std::string embedded = wrote("embedded",
        R"({"networkID":3,"cChainGenesis":"{\"config\":{\"chainId\":200202}}"})");
    ok(accepted(embedded, zoo::kDevnet), "and one carrying the chain as a string");
    ok(accepted(mine, zoo::kMainnet), "and a chain document that names no network");

    // ── documents that must not be believed ─────────────────────────────────
    // The chain is this network's and the network is not: a chain document
    // carried inside another network's genesis belongs to that network.
    const std::string elsewhere = wrote("elsewhere",
        R"({"networkID":9,"cChainGenesis":{"config":{"chainId":200200}}})");
    ok(!accepted(elsewhere, zoo::kMainnet), "a document naming another network is refused");
    // And the network is this one while the chain is not.
    const std::string crossed = wrote("crossed",
        R"({"networkID":1,"cChainGenesis":{"config":{"chainId":200201}}})");
    ok(!accepted(crossed, zoo::kMainnet), "and so is one carrying another chain");
    ok(refused(wrote("quotednet",
        R"({"networkID":"1","cChainGenesis":{"config":{"chainId":200200}}})")),
       "a quoted network id is refused, not read as a number");
    ok(refused(wrote("torn", R"({"config":{"chainId":200200)")), "truncated JSON is refused");
    ok(refused(wrote("empty", "")), "an empty file is refused");
    ok(refused(wrote("bare", R"([1,2,3])")), "a document that is not an object is refused");
    ok(refused(wrote("noconfig", R"({"alloc":{}})")), "a document with no config is refused");
    ok(refused(wrote("noid", R"({"config":{}})")), "a config with no chain id is refused");
    ok(refused(wrote("quoted", R"({"config":{"chainId":"200200"}})")),
       "a quoted chain id is refused, not read as a number");
    ok(refused(wrote("signed", R"({"config":{"chainId":-1}})")), "a negative chain id is refused");
    ok(refused(wrote("badalloc", R"({"config":{"chainId":200200},"alloc":[]})")),
       "an allocation that is not a set of accounts is refused");
    ok(refused("/nonexistent/genesis.json"), "a file that is not there is refused");

    // A document that wraps itself must stop rather than recurse.
    std::string loop = R"({"config":{"chainId":200200}})";
    for (int i = 0; i < 12; ++i) loop = R"({"cChainGenesis":)" + loop + "}";
    ok(refused(wrote("loop", loop)), "a document nested past the limit is refused");

    std::cout << (failures ? "FAILED " : "passed ") << failures << " failing\n";
    return failures ? 1 : 0;
}
