// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// SPDX-License-Identifier: BSD-3-Clause-Eco
//
// The networks this node runs, and the two numbers each one is.
//
// ONE NUMBER PER NETWORK PER ENVIRONMENT, because Zoo is a sovereign L1.
//
// A validator joins a NETWORK and a transaction is signed against a CHAIN, and
// on a network carrying many chains those are two questions with two answers.
// Zoo's primary network carries one: its EVM. So the two numbers were free to
// differ, and differing is what went wrong — mainnet greeted under id 1, which
// is what Lux mainnet greets under, and so did Hanzo's. Three sovereign
// networks introducing themselves by the same number is an ambiguity a wallet
// cannot see through and a staking-key derivation path cannot be unique under.
//
// So the primary network id IS the EVM chain id. One number per environment,
// globally unique, and a peer greeting under 200200 is on Zoo mainnet and
// nowhere else.
//
// These three are the whole set. A network that is not here is not one this
// binary can be pointed at by name — which is the point of compiling them in
// rather than reading them from a file that can be edited into a fork.

#pragma once

#include <cstdint>
#include <optional>
#include <string>
#include <string_view>

namespace zoo {

// One network: what it is called, the number its validators greet under, and
// the chain this node serves on it.
struct Network {
    std::string_view name;   // the name a person uses
    std::uint32_t    id;     // what peers greet each other under
    std::uint64_t    chain;  // what a transaction on it is signed against
};

// The settlement network.
inline constexpr Network kMainnet{"mainnet", 200200, 200200};
// The network that carries release candidates.
inline constexpr Network kTestnet{"testnet", 200201, 200201};
// The network the protocol itself is developed against.
inline constexpr Network kDevnet{"devnet", 200202, 200202};

// Every network this binary knows, in the order it lists them.
inline constexpr Network kAll[]{kMainnet, kTestnet, kDevnet};

// The network called `name`, or nothing.
//
// Nothing, rather than a default: a node that fell back to a network when it
// was asked for one it did not have would be a node that joins the wrong
// network on a typo, and it would look like it had started correctly.
std::optional<Network> network_named(std::string_view name);

// The names, for a message that has to list them.
std::string network_names();

}  // namespace zoo
