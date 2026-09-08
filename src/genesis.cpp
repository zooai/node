// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// SPDX-License-Identifier: BSD-3-Clause-Eco

#include "genesis.h"

#include <nlohmann/json.hpp>

#include <fstream>
#include <sstream>
#include <stdexcept>

namespace zoo {
namespace {

using Json = nlohmann::json;

// How deep a `cChainGenesis` chain is followed before it is called a loop.
// A document that wraps itself would otherwise recurse until the stack ends.
constexpr int kMaxNesting = 8;

// Read a number that must be a number. A quoted or absent value is an error
// here rather than a zero three lines later.
std::uint64_t number(const Json& at, const char* field) {
    const auto found = at.find(field);
    if (found == at.end()) throw std::runtime_error(std::string("no ") + field);
    if (!found->is_number_unsigned())
        throw std::runtime_error(std::string(field) + " is not a whole number");
    return found->get<std::uint64_t>();
}

// Descend to the chain's own document, wherever it is written.
const Json* chain_document(const Json& doc, Json& held, int depth) {
    if (depth > kMaxNesting) throw std::runtime_error("cChainGenesis nests too deeply");
    const auto inner = doc.find("cChainGenesis");
    if (inner == doc.end()) return &doc;
    if (inner->is_string()) {
        held = Json::parse(inner->get<std::string>(), nullptr, false);
        if (held.is_discarded()) throw std::runtime_error("cChainGenesis is not JSON");
        return chain_document(held, held, depth + 1);
    }
    return chain_document(*inner, held, depth + 1);
}

}  // namespace

void Genesis::agrees_with(const Network& net) const {
    if (chain != net.chain) {
        std::ostringstream why;
        why << "this genesis is chain " << chain << ", not " << net.name << "'s chain "
            << net.chain;
        throw std::runtime_error(why.str());
    }
    // A chain document carried inside another network's genesis is that
    // network's, whatever its chain id says.
    if (network && *network != net.id) {
        std::ostringstream why;
        why << "this genesis is network " << *network << ", not " << net.name << " ("
            << net.id << ")";
        throw std::runtime_error(why.str());
    }
}

Genesis read_genesis(const std::string& path) {
    std::ifstream file(path);
    if (!file) throw std::runtime_error("cannot read it");

    Json doc = Json::parse(file, nullptr, false);
    if (doc.is_discarded()) throw std::runtime_error("not JSON");
    if (!doc.is_object()) throw std::runtime_error("not a genesis document");

    Json held;
    const Json& inner = *chain_document(doc, held, 0);
    if (!inner.is_object()) throw std::runtime_error("cChainGenesis is not a document");

    const auto config = inner.find("config");
    if (config == inner.end() || !config->is_object())
        throw std::runtime_error("no config");
    const std::uint64_t chain = number(*config, "chainId");

    // The network the document names, when it names one. Read here and judged
    // in agrees_with, so this function answers what the document says and the
    // other answers whether it is ours.
    std::optional<std::uint64_t> network;
    if (doc.find("networkID") != doc.end()) network = number(doc, "networkID");

    std::size_t accounts = 0;
    if (const auto alloc = inner.find("alloc"); alloc != inner.end()) {
        if (!alloc->is_object()) throw std::runtime_error("alloc is not a set of accounts");
        accounts = alloc->size();
    }
    return Genesis{chain, network, accounts};
}

}  // namespace zoo
