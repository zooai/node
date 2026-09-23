# syntax=docker/dockerfile:1.26@sha256:ecfaec9ed6d810b56388c508f4121597bfbba70d41a6dfeee4d8cad5f295fc32
#
# zood — the Zoo network node, compiled from source.
#
# zood is lux-cpp/node's own daemon, src/noded.cpp, built under Zoo's name,
# endpoint and chain id 200200 (CMakeLists.txt). Everything it links is built
# here from the commits pinned below, so one commit of this repository names one
# binary:
#
#   luxcrypto  luxfi/crypto `make dist`: libluxcrypto.a, the ML-DSA-65 and
#              ML-KEM-768 of the post-quantum peer handshake, as a Go c-archive
#   builder    AWS-LC, cevm's Conan dependencies, the consensus engine, cevm and
#              lux-cpp/node, then zood
#   runtime    gcr.io/distroless/cc-debian12:nonroot and the one binary
#
# Private repositories are read with the BuildKit secret GH_READ_TOKEN, a GitHub
# token that can read them. It is never an ARG and never a file: the steps that
# fetch give it to git as configuration in their own environment, and each fails
# if the token turns up in anything it wrote.
#
# Every Debian here is 12, the runtime's. A binary carries the glibc symbol
# versions it was linked against; linked against a newer glibc it builds, it
# pushes, and it dies on first exec in distroless-debian12.
#
# Nothing here names an architecture. The runner builds each platform on a node
# of that platform, and this file compiles for whichever it is on.

# ── pins ────────────────────────────────────────────────────────────────────
# Full commit SHAs. A branch name is refused: it is a different tree tomorrow.

# lux-cpp/node main: a node answers for the chains its own network owns, so
# zood serves /v1/chain/zoo and /v1/chain/200200 and 404s /v1/chain/c.
ARG LUX_CPP_NODE=5e718314f09ee6fddd77ef30f704a464e9d8212a
ARG LUX_CPP_CONSENSUS=9928599fc95d92c54b759468a351af907ab29aed
# cevm main, the tree lux-cpp/node's tests pass on and its own image builds
# against. Not the integrate/fixes/fix-* branches: those are unmerged GPU-EVM
# work, and this is the CPU build.
ARG LUX_CPP_CEVM=1c7e94f0afc61cf39c7afb411360f9a0b460de9e
ARG LUX_CPP_CRYPTO=f5f597d638ad4e00abbb250b007f355bd1910fe8
ARG LUX_CPP_BLST=0bf3bb96c630d7451b8549d1ad95074997a5a5e3
ARG LUX_CPP_ZAP=2a37db5a574f73d972c8c22e5d79b6b24dcbe637
ARG LUX_GPU_KERNELS=76cdda14b381f77cc42d81c094460bbce6bcb3d1
# The last luxfi/crypto on Go 1.26.8, which lux-cpp/node's last green image
# linked. The commit after it changes only the go directive, to 1.27.1.
ARG LUXFI_CRYPTO=923186d4163bf15acf537f96b679a1f7f7915a64
# aws-lc v1.65.0.
ARG AWS_LC=7b627926398b6bb786296132b22dbb37bab7649f

# ── fetch ───────────────────────────────────────────────────────────────────
# Sourced by every step that fetches. It holds no secret.
FROM scratch AS scripts
COPY <<'EOF' /fetch.sh
# auth: read access to GitHub for the rest of this RUN, from GH_READ_TOKEN.
# git reads it as configuration from its environment (GIT_CONFIG_COUNT, see
# git-config(1)), so no file records it — not .git/config, not ~/.gitconfig,
# not a layer — while every git this RUN starts inherits it: submodule clones,
# and the FetchContent clones inside Conan's builds. It is sent to
# https://github.com/ and to no other host.
auth() {
  if [ -r /run/secrets/GH_READ_TOKEN ]; then
    token=$(tr -d '[:space:]' < /run/secrets/GH_READ_TOKEN)
  else
    token=
  fi
  if [ -z "$token" ]; then
    echo "zood: the build secret GH_READ_TOKEN is missing. It is a GitHub token that can read the private lux-cpp, lux-gpu and luxfi repositories this build fetches; pass it as --secret id=GH_READ_TOKEN." >&2
    exit 1
  fi
  header=$(printf 'x-access-token:%s' "$token" | base64 -w0)
  export GIT_CONFIG_COUNT=1 \
    GIT_CONFIG_KEY_0=http.https://github.com/.extraheader \
    GIT_CONFIG_VALUE_0="AUTHORIZATION: basic $header"
}

# sealed <dir>...: refuses the layer if the token, bare or as the header, is in
# any file under <dir>. Checked, not assumed.
sealed() {
  if grep -rqsF -e "$token" -e "$header" -- "$@"; then
    echo "zood: the GitHub token was written under $*; the layer is refused." >&2
    exit 1
  fi
}

# fetch <owner/repo> <sha> <dir> [submodule...]: the one commit, by its full
# SHA, and no history.
fetch() {
  repo=$1 sha=$2 dir=$3
  shift 3
  if ! printf '%s' "$sha" | grep -Eqx '[0-9a-f]{40}'; then
    echo "zood: $repo is pinned to '$sha', which is not a full commit SHA." >&2
    exit 1
  fi
  git init -q "$dir"
  git -C "$dir" remote add origin "https://github.com/$repo"
  git -C "$dir" fetch -q --depth 1 origin "$sha"
  git -C "$dir" checkout -q --detach FETCH_HEAD
  test "$(git -C "$dir" rev-parse HEAD)" = "$sha"
  if [ $# -gt 0 ]; then
    git -C "$dir" submodule update -q --init --depth 1 -- "$@"
  fi
}
EOF

# ── luxcrypto ───────────────────────────────────────────────────────────────
# A Go c-archive, so Go builds it and the C++ stage takes the archive. Public
# sources and modules: this stage is given no token.
FROM golang:1.26.8-bookworm@sha256:a688600ca24f8a4d3ca77f95b0dd40704a9fc787c826660eb7ba0b641b8b175d AS luxcrypto
COPY --from=scripts /fetch.sh /usr/local/lib/fetch.sh
ARG LUXFI_CRYPTO
RUN <<'EOF'
set -eu
. /usr/local/lib/fetch.sh
fetch luxfi/crypto "$LUXFI_CRYPTO" /src/lux/crypto
cd /src/lux/crypto
go mod download github.com/luxfi/accel
make dist
test -s dist/libluxcrypto.a
EOF

# ── builder ─────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim@sha256:3783cc01769c7b2b1b83a5c5ad96c815348e28ed7da68e2e3687004faa906251 AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential cmake ninja-build git ca-certificates go-md2man golang-go perl \
        nlohmann-json3-dev python3 python3-pip python3-venv \
    && rm -rf /var/lib/apt/lists/*

# cevm resolves intx, blst, lux-crypto and lux-zap-core through Conan.
RUN python3 -m venv /opt/conan && /opt/conan/bin/pip install --no-cache-dir conan==2.32.0
ENV PATH=/opt/conan/bin:$PATH

COPY --from=scripts /fetch.sh /usr/local/lib/fetch.sh

# AWS-LC: the TLS 1.3 + X25519MLKEM768 the peer link runs on. Static, built once.
ARG AWS_LC
RUN <<'EOF'
set -eu
. /usr/local/lib/fetch.sh
fetch aws/aws-lc "$AWS_LC" /src/aws-lc
cmake -S /src/aws-lc -B /src/aws-lc-build -G Ninja \
  -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DBUILD_TESTING=OFF
cmake --build /src/aws-lc-build --target ssl crypto
EOF

# cevm's dependencies. The lux packages are on no Conan remote, so they are
# exported from their checkouts first. lux-crypto's own dependencies are CMake
# FetchContent clones of private forks, which is why this step needs the token
# past the fetch. blst is built portable, dispatching ADX at run time, so the
# binary does not depend on the CPU of the node that happened to build it.
ARG LUX_CPP_CEVM LUX_CPP_CRYPTO LUX_CPP_BLST LUX_CPP_ZAP
RUN --mount=type=secret,id=GH_READ_TOKEN <<'EOF'
set -eu
. /usr/local/lib/fetch.sh
auth
# evmc only: cevm refuses to configure without it. test/evm-benchmarks is tests.
fetch lux-cpp/cevm         "$LUX_CPP_CEVM"   /src/luxcpp/cevm evmc
fetch lux-cpp/crypto       "$LUX_CPP_CRYPTO" /src/luxcpp/crypto
fetch lux-cpp/blst         "$LUX_CPP_BLST"   /src/luxcpp/blst
fetch lux-cpp/zap-cpp-core "$LUX_CPP_ZAP"    /src/luxcpp/zap-cpp-core
conan profile detect --force
printf 'core.download:parallel=1\n' >> "$(conan config home)/global.conf"
conan export /src/luxcpp/crypto
conan export /src/luxcpp/blst
conan export /src/luxcpp/zap-cpp-core
conan install /src/luxcpp/cevm \
  -pr /src/luxcpp/cevm/.github/conan/manylinux-relax.profile \
  -s build_type=Release -s compiler.cppstd=gnu20 \
  -o 'bls/*:portable=True' \
  --output-folder=/src/cevm-conan --build=missing
sealed /src /root /tmp
EOF

# The node and the engine under it. gpu-kernels goes where cevm's CMakeLists
# name it, lux-private/gpu-kernels four levels above lib/evm: it carries the
# 0x100 precompile's 256-bit arithmetic and the aivm oracle.
ARG LUX_CPP_CONSENSUS LUX_GPU_KERNELS LUX_CPP_NODE
RUN --mount=type=secret,id=GH_READ_TOKEN <<'EOF'
set -eu
. /usr/local/lib/fetch.sh
auth
fetch lux-cpp/consensus   "$LUX_CPP_CONSENSUS" /src/lux-cpp/consensus
fetch lux-gpu/gpu-kernels "$LUX_GPU_KERNELS"   /src/lux-private/gpu-kernels
fetch lux-cpp/node        "$LUX_CPP_NODE"      /src/lux-cpp/node
sealed /src /root /tmp
EOF

# Where lux-cpp/node looks for it: ../../lux/crypto/dist from the top source
# directory, which is this repository at /src/zooai/node.
COPY --from=luxcrypto /src/lux/crypto/dist /src/lux/crypto/dist

COPY CMakeLists.txt /src/zooai/node/
COPY src /src/zooai/node/src
COPY test /src/zooai/node/test

# Release, stripped at link time. Then three checks before anything ships: the
# binary carries Zoo's name rather than the host's default; it needs no library
# the runtime lacks (a Conan .so, or a system libssl in place of AWS-LC, would
# pass here and fail on first exec there); and it runs. --publish on a
# throwaway directory makes an ML-DSA-65 identity through libluxcrypto's Go
# runtime and a BLS key, and prints the committee line the BLS key signs.
RUN <<'EOF'
set -eu
cmake -S /src/zooai/node -B /src/build -G Ninja \
  -DCMAKE_TOOLCHAIN_FILE=/src/cevm-conan/build/Release/generators/conan_toolchain.cmake \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_CXX_FLAGS_RELEASE="-O3 -DNDEBUG" \
  -DCMAKE_EXE_LINKER_FLAGS="-s" \
  -DLUX_NODE_DIR=/src/lux-cpp/node \
  -DCONSENSUS_DIR=/src/lux-cpp/consensus \
  -DCEVM_DIR=/src/luxcpp/cevm \
  -DLUXCPP_ROOT=/src/luxcpp \
  -DZAP_DIR=/src/luxcpp/zap-cpp-core/include \
  -DAWSLC_SRC=/src/aws-lc \
  -DAWSLC_BUILD_DIR=/src/aws-lc-build \
  -DBLST_ISA=portable
cmake --build /src/build --target zood
z=/src/build/zood
if ! grep -qF 'zooai/zood/v0.1.0' "$z"; then
  echo "zood: the binary does not answer as zooai/zood; the brand did not reach noded.cpp." >&2
  exit 1
fi
needs=$(readelf -d "$z" | sed -n 's/.*(NEEDED).*\[\(.*\)\]$/\1/p')
test -n "$needs"
extra=$(printf '%s\n' "$needs" | grep -Evx 'lib(c|m|pthread|dl|rt|resolv|gcc_s|stdc\+\+|gomp)\.so\.[0-9]+|ld-linux-(x86-64|aarch64)\.so\.[0-9]+' || true)
if [ -n "$extra" ]; then
  echo "zood: links $extra, which gcr.io/distroless/cc-debian12 does not carry." >&2
  exit 1
fi
k=$(mktemp -d)
"$z" --data "$k" --publish > "$k.line"
test -s "$k.line"
rm -rf "$k" "$k.line"
EOF

# ── runtime ─────────────────────────────────────────────────────────────────
# cc carries exactly the C and C++ runtimes zood links, and no shell, package
# manager or fetcher: an image that holds a validator's key holds nothing that
# can be told to fetch and run.
FROM gcr.io/distroless/cc-debian12:nonroot@sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f
COPY --from=builder /src/build/zood /usr/local/bin/zood
# The JSON-RPC and the validator mesh. A validator is named by the key it proves
# on every link, so the mesh port is not an admin surface and needs no gate.
EXPOSE 9630 9631
USER nonroot
ENTRYPOINT ["/usr/local/bin/zood"]
