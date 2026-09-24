// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// SPDX-License-Identifier: BSD-3-Clause-Eco
//
// The genesis each Zoo chain starts from, compiled in from genesis/*.json.
//
// mainnet and testnet are the documents their history starts from — luxfi/state
// chains/zoo-{mainnet,testnet}/genesis.json, block 0x7c548af4… and 0x0652fb2f…
// — so a node reading luxfi/state's exports finds block 1's parent at home.
// devnet is a new chain.

#pragma once

namespace zoo::genesis {

extern const char* const kMainnet;
extern const char* const kTestnet;
extern const char* const kDevnet;

}  // namespace zoo::genesis
