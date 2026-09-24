// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// SPDX-License-Identifier: BSD-3-Clause-Eco
//
// zood — the Zoo network's node: lux-cpp/node run on Zoo's specs. Zoo is an L2
// on Lux's validator set, so its networks greet under Lux's network ids and its
// chains are its own.

#include "genesis.h"

#include <lux/node/spec.hpp>

#include <array>

namespace {

constexpr const char* kClient   = "zooai/zood/v0.1.4";
constexpr const char* kEndpoint = "https://api.zoo.ngo";
constexpr const char* kVm       = "/usr/local/libexec/lux/cevm";

const std::array<lux::node::Spec, 3> kSpecs{{
    {"mainnet", kClient, kEndpoint, 1, 200200, zoo::genesis::kMainnet, kVm},
    {"testnet", kClient, kEndpoint, 2, 200201, zoo::genesis::kTestnet, kVm},
    {"devnet", kClient, kEndpoint, 3, 200202, zoo::genesis::kDevnet, kVm},
}};

}  // namespace

int main(int argc, char** argv) { return lux::node::run(kSpecs, argc, argv); }
