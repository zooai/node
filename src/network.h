// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// SPDX-License-Identifier: BSD-3-Clause-Eco
//
// The networks this node runs, and what each one is called by number.
//
// A sovereign network is named by ONE number. It is the network id peers greet
// each other with and it is the EVM chain id a transaction is signed against,
// and they are the same number on purpose: two numbers for one network is a way
// for a wallet and a validator to disagree about which network they are on, and
// the disagreement is only visible after a transaction has been signed for the
// wrong one.
//
// These three are the whole set. A network that is not here is not one this
// binary can be pointed at by name — which is the point of compiling them in
// rather than reading them from a file that can be edited into a fork.

#pragma once

#include <cstdint>
#include <optional>
#include <span>
#include <string>
#include <string_view>

namespace zoo {

// One network: what it is called, and the number it is.
struct Network {
    std::string_view name;  // the name a person uses
    std::uint64_t    id;    // network id and EVM chain id, one value
};

// The settlement network.
inline constexpr Network kMainnet{"mainnet", 200200};
// The network that carries release candidates.
inline constexpr Network kTestnet{"testnet", 200201};
// The network the protocol itself is developed against.
inline constexpr Network kDevnet{"devnet", 200202};

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
