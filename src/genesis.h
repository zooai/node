// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// SPDX-License-Identifier: BSD-3-Clause-Eco
//
// Reading a genesis document, and holding it to the network it claims to be.
//
// A genesis is data, and this is the one place it becomes a value this node
// will act on. What is refused matters more than what is read: a field that is
// present and unreadable is an error, never a default. A document that fell
// back to zero for a number it could not parse would agree with nothing and
// look like it had been checked.

#pragma once

#include <cstddef>
#include <cstdint>
#include <string>

#include "network.h"

namespace zoo {

// What a genesis document says about which network it is for.
struct Genesis {
    // The number the document states — its EVM chain id, and for a sovereign
    // network its network id too.
    std::uint64_t id;
    // How many accounts it allocates.
    std::size_t accounts;

    // Refuse this document if it is not this network's.
    //
    // Throws rather than returns, because there is nothing a caller can
    // sensibly do with a genesis for another network except stop.
    void agrees_with(const Network& net) const;
};

// Read the genesis document at `path`.
//
// Both shapes on disk are accepted, because both are what is written: the
// chain's own document, or a whole-network document carrying it under
// `cChainGenesis` — as an object, or as a string holding the document.
//
// Throws std::runtime_error naming what was wrong with it.
Genesis read_genesis(const std::string& path);

}  // namespace zoo
