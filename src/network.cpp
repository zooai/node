// Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
// SPDX-License-Identifier: BSD-3-Clause-Eco

#include "network.h"

#include <algorithm>
#include <iterator>

namespace zoo {

std::optional<Network> network_named(std::string_view name) {
    const auto* found = std::find_if(std::begin(kAll), std::end(kAll),
                                     [&](const Network& n) { return n.name == name; });
    if (found == std::end(kAll)) return std::nullopt;
    return *found;
}

std::string network_names() {
    std::string names;
    for (const Network& n : kAll) {
        if (!names.empty()) names += " | ";
        names += n.name;
    }
    return names;
}

}  // namespace zoo
