# Copyright (C) 2026, Zoo Labs Foundation. All rights reserved.
# SPDX-License-Identifier: BSD-3-Clause-Eco
#
# zood — the Zoo network node.

cmake_minimum_required(VERSION 3.24)
project(zood VERSION 0.1.3 LANGUAGES C CXX ASM)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Release)
endif()

# The node this one is. Identity, the validator mesh and its post-quantum
# handshake, the block engine, the EVM (cevm) and the JSON-RPC server all come
# from here, at a tag: two nodes on one network resolve the same tag or they do
# not agree about what a block means. A second copy of any of it in this
# repository would be a second implementation of the thing every peer has to
# match — which is what this repository had, and why it served nothing.
# A checkout beside this one is used when given, and fetched at the pinned tag
# when it is not, so the image build (which already carries the host it was
# built on) and a bare clone resolve the same node either way.
include(FetchContent)
set(LUX_NODE_DIR "" CACHE PATH "a lux-cpp/node checkout to build against")
if(LUX_NODE_DIR)
  message(STATUS "zood: node = ${LUX_NODE_DIR}")
  add_subdirectory(${LUX_NODE_DIR} ${CMAKE_BINARY_DIR}/lux-node EXCLUDE_FROM_ALL)
else()
  FetchContent_Declare(lux_node
    GIT_REPOSITORY https://github.com/lux-cpp/node.git
    GIT_TAG        main
    FIND_PACKAGE_ARGS)
  FetchContent_MakeAvailable(lux_node)
endif()

# Zoo's genesis documents, compiled in: a network is not something a file
# edited on a host can turn into a fork.
foreach(_n MAINNET TESTNET DEVNET)
  string(TOLOWER ${_n} _f)
  file(READ ${CMAKE_CURRENT_SOURCE_DIR}/genesis/${_f}.json ZOO_GENESIS_${_n})
  set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS genesis/${_f}.json)
endforeach()
configure_file(src/genesis.cpp.in ${CMAKE_BINARY_DIR}/genesis.cpp @ONLY)

# zood is lux-cpp/node run on Zoo's specs (src/zood.cpp), and cevm is the EVM
# plugin its chain runs in.
add_executable(zood src/zood.cpp ${CMAKE_BINARY_DIR}/genesis.cpp)
target_include_directories(zood PRIVATE src)
target_link_libraries(zood PRIVATE node)
target_compile_options(zood PRIVATE -Wall -Wextra -Wpedantic -Werror)
add_dependencies(zood cevm-bin)

include(GNUInstallDirs)
install(TARGETS zood RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
