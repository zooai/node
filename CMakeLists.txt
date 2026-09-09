# Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
# SPDX-License-Identifier: BSD-3-Clause-Eco
#
# zood — the Zoo network node.

cmake_minimum_required(VERSION 3.24)
project(zood VERSION 0.1.0 LANGUAGES C CXX ASM)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Release)
endif()

# The JSON reader a genesis document is read with. One declaration: the
# installed package is used when the host has it, and fetched at the pinned tag
# when it does not, so the build is the same build either way.
include(FetchContent)
FetchContent_Declare(nlohmann_json
  GIT_REPOSITORY https://github.com/nlohmann/json.git
  GIT_TAG        v3.11.3
  FIND_PACKAGE_ARGS 3)
FetchContent_MakeAvailable(nlohmann_json)

# The node this one is. Identity, the validator mesh and its post-quantum
# handshake, the block engine, the EVM (cevm) and the JSON-RPC server all come
# from here, at a tag: two nodes on one network resolve the same tag or they do
# not agree about what a block means. A second copy of any of it in this
# repository would be a second implementation of the thing every peer has to
# match — which is what this repository had, and why it served nothing.
FetchContent_Declare(lux_node
  GIT_REPOSITORY https://github.com/lux-cpp/node.git
  GIT_TAG        main
  FIND_PACKAGE_ARGS)
FetchContent_MakeAvailable(lux_node)

# The network's identity and the documents checked against it. A library rather
# than loose sources, so the tests exercise exactly what the daemon runs.
add_library(network STATIC src/network.cpp src/genesis.cpp)
target_include_directories(network PUBLIC src)
target_link_libraries(network PUBLIC nlohmann_json::nlohmann_json)
target_compile_options(network PRIVATE -Wall -Wextra -Wpedantic -Werror)

add_executable(zood src/main.cpp)
target_link_libraries(zood PRIVATE network node)
target_compile_options(zood PRIVATE -Wall -Wextra -Wpedantic -Werror)

enable_testing()
add_executable(check test/check.cpp)
target_link_libraries(check PRIVATE network)
target_compile_options(check PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_test(NAME check COMMAND check)

include(GNUInstallDirs)
install(TARGETS zood RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
