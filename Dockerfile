# syntax=docker/dockerfile:1.26@sha256:ecfaec9ed6d810b56388c508f4121597bfbba70d41a6dfeee4d8cad5f295fc32
#
# zood — the Zoo network node, compiled from source.
#
# zood is lux-cpp/node run on Zoo's specs (src/zood.cpp, genesis/), and cevm
# is the EVM plugin its chain runs in. Every source it compiles is fetched here
# at a commit pinned below:
#
#   luxcrypto  luxfi/crypto `make dist`: libluxcrypto.a, the ML-DSA-65 and
#              ML-KEM-768 of the post-quantum peer handshake, as a Go c-archive
#   builder    AWS-LC, cevm's Conan dependencies, the consensus engine, cevm and
#              lux-cpp/node, then zood and cevm
#   runtime    gcr.io/distroless/cc-debian12:nonroot and the two binaries
#
# What is not pinned by commit resolves when the image is built: Debian's
# packages, Conan's Python dependencies, the intx, nlohmann_json and cmake
# packages from ConanCenter, and blst v0.3.14, which lux-crypto's kzg build
# clones by tag for its headers.
#
# Private repositories are read with the BuildKit secret GH_READ_TOKEN, a GitHub
# token that can read them. It is never an ARG and never a file. Only the two
# steps that fetch mount it, and they run git and nothing else: git takes it as
# configuration from their environment, and each fails if the token is anywhere
# in the filesystem it leaves. The steps that run other people's code, Conan's
# above all, do not mount it; that keeps it from them only where the builder
# isolates one step's processes from the daemon that holds the secret.
#
# Every Debian here is 12, the runtime's. A binary carries the glibc symbol
# versions it was linked against; linked against a newer glibc it builds, it
# pushes, and it dies on first exec in distroless-debian12.
#
# Nothing here names an architecture. The runner builds each platform on a node
# of that platform, and this file compiles for whichever it is on.

# ── pins ────────────────────────────────────────────────────────────────────
# Full commit SHAs, each checked to be on the branch or tag its fetch line
# names, spelled in full: a bare `main` would also match a tag called main. GitHub serves any commit it still holds by SHA, including ones no
# branch reaches any more and ones that only exist in a fork.

# run(spec) and the EVM run as a plugin. zood serves /v1/chain/zoo and
# /v1/chain/200200, 404s /v1/chain/c, and binds --rpc-host (127.0.0.1 unless
# told otherwise), so a pod passes --rpc-host 0.0.0.0.
ARG LUX_CPP_NODE=1030ab79cd911e15cc6eb4524effcc7e2a6a09db
ARG LUX_CPP_CONSENSUS=9928599fc95d92c54b759468a351af907ab29aed
# cevm main, the tree lux-cpp/node's tests pass on and its own image builds
# against. Not the integrate/fixes/fix-* branches: those are unmerged GPU-EVM
# work, and this is the CPU build.
ARG LUX_CPP_CEVM=0feba60c0e04c25a4ef069bc1897ad91adbb8fba
# The evmc commit cevm's submodule names, fetched where the submodule goes.
ARG LUX_CPP_EVMC=7cb4b0ca30df7b036d1a582196af5e9a1455a47d
# The tree lux-cpp/node's last green image built. The commit after it on main
# adds bcc to CMake's algorithm list and not to the Conan recipe's exports, so
# the package it makes cannot configure.
ARG LUX_CPP_CRYPTO=87e59776b97e9e13f1f07ee4deb129f9cda8cff8
ARG LUX_CPP_BLST=0bf3bb96c630d7451b8549d1ad95074997a5a5e3
ARG LUX_CPP_ZAP=2a37db5a574f73d972c8c22e5d79b6b24dcbe637
ARG LUX_GPU_KERNELS=76cdda14b381f77cc42d81c094460bbce6bcb3d1
# lux-crypto's CMake FetchContent dependencies, at the commits its tags name.
ARG LUX_CPP_INTX=0306b495820a5259563e5055f0a8b726b9ae4e0b
ARG LUX_CPP_EVMMAX=9af2f9ac9b45fef6d547e59a06ea091f64966700
ARG LUX_CPP_PQCLEAN=826eff8124946a005c189eaf2aba5d2a3df8329a
ARG LUX_CPP_ED25519_DONNA=805dc05f47d3dfaa574cea16fd78321d63b6b17d
ARG LUX_CPP_BLAKE3_REFERENCE=38173020aa1f643c904f8f0f0340e16f31f01cfc
ARG LUXFI_C_KZG_4844=9763b8f10ea45221f383e6f6500b78eb4e4ad748
ARG LUXFI_SR25519_CRUST=0cb4dc4d96e20c4b3dd6d520a43544ea55921ec4
# The last luxfi/crypto whose go directive golang:1.26.8 builds. main after it
# moves the directive to 1.27.1 and changes no Go source.
ARG LUXFI_CRYPTO=923186d4163bf15acf537f96b679a1f7f7915a64
ARG AWS_LC=7b627926398b6bb786296132b22dbb37bab7649f

# ── fetch ───────────────────────────────────────────────────────────────────
# Sourced by every step that fetches. It holds no secret.
FROM scratch AS scripts
COPY <<'EOF' /fetch.sh
# auth: read access to GitHub for the rest of this RUN, from GH_READ_TOKEN.
# git reads it as configuration from its environment (GIT_CONFIG_COUNT, see
# git-config(1)), so no file records it — not .git/config, not ~/.gitconfig,
# not a layer. It is sent to https://github.com/ and to no other host.
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

# sealed: refuses the layer if the token, bare or as the header, is in any file
# on this filesystem, /run/secrets aside, where the secret itself is mounted. A
# file that cannot be read refuses it too: unread is not clean.
sealed() {
  set --
  for d in /* /.[!.]* /run/*; do
    case $d in
      /proc | /sys | /dev | /run | /run/secrets) ;;
      *) [ -e "$d" ] && [ ! -L "$d" ] && set -- "$@" "$d" ;;
    esac
  done
  found=0
  grep -rqF -D skip -e "$token" -e "$header" -- "$@" || found=$?
  case $found in
    0) echo "zood: the GitHub token was written into this layer; it is refused." >&2
       exit 1 ;;
    1) ;;
    *) echo "zood: could not read every file this layer holds; it is refused." >&2
       exit 1 ;;
  esac
}

# fetch <owner/repo> <refs/heads/… or refs/tags/…> <sha> <dir>: the one
# commit, by its full SHA, with no history, after proving it is on that branch
# or tag. The proof reads commits only, in a repository of its own that is then
# removed.
fetch() {
  repo=$1 ref=$2 sha=$3 dir=$4
  if ! printf '%s' "$sha" | grep -Eqx '[0-9a-f]{40}'; then
    echo "zood: $repo is pinned to '$sha', which is not a full commit SHA." >&2
    exit 1
  fi
  case $ref in
    refs/heads/?* | refs/tags/?*) ;;
    *) echo "zood: $repo is checked against '$ref', which is not a full branch or tag ref." >&2
       exit 1 ;;
  esac
  log=$(mktemp -d)
  git init -q --bare "$log"
  git -C "$log" remote add origin "https://github.com/$repo"
  git -C "$log" fetch -q --filter=tree:0 origin "$ref"
  if ! git -C "$log" rev-list FETCH_HEAD | grep -qx "$sha"; then
    echo "zood: $repo $sha is not on $ref." >&2
    exit 1
  fi
  rm -rf "$log"
  git init -q "$dir"
  git -C "$dir" remote add origin "https://github.com/$repo"
  git -C "$dir" fetch -q --depth 1 origin "$sha"
  git -C "$dir" checkout -q --detach FETCH_HEAD
  test "$(git -C "$dir" rev-parse HEAD)" = "$sha"
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
fetch luxfi/crypto refs/heads/main "$LUXFI_CRYPTO" /src/lux/crypto
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
fetch aws/aws-lc refs/tags/v1.65.0 "$AWS_LC" /src/aws-lc
cmake -S /src/aws-lc -B /src/aws-lc-build -G Ninja \
  -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DBUILD_TESTING=OFF
cmake --build /src/aws-lc-build --target ssl crypto
EOF

# What Conan builds from. lux-crypto's CMake would clone seven more private
# repositories while it configures, so they are fetched here instead, into
# /src/deps under their FetchContent names, and the next step points FetchContent
# at them. evmc goes where cevm's submodule does; cevm refuses to configure
# without it.
ARG LUX_CPP_CEVM LUX_CPP_EVMC LUX_CPP_CRYPTO LUX_CPP_BLST LUX_CPP_ZAP \
    LUX_CPP_INTX LUX_CPP_EVMMAX LUX_CPP_PQCLEAN LUX_CPP_ED25519_DONNA \
    LUX_CPP_BLAKE3_REFERENCE LUXFI_C_KZG_4844 LUXFI_SR25519_CRUST
RUN --mount=type=secret,id=GH_READ_TOKEN <<'EOF'
set -eu
. /usr/local/lib/fetch.sh
auth
fetch lux-cpp/cevm             refs/heads/main    "$LUX_CPP_CEVM"             /src/luxcpp/cevm
fetch lux-cpp/evmc             refs/heads/main    "$LUX_CPP_EVMC"             /src/luxcpp/cevm/evmc
fetch lux-cpp/crypto           refs/heads/main    "$LUX_CPP_CRYPTO"           /src/luxcpp/crypto
fetch lux-cpp/blst             refs/heads/main    "$LUX_CPP_BLST"             /src/luxcpp/blst
fetch lux-cpp/zap-cpp-core     refs/heads/main    "$LUX_CPP_ZAP"              /src/luxcpp/zap-cpp-core
fetch lux-cpp/intx             refs/tags/v0.15.0 "$LUX_CPP_INTX"             /src/deps/luxcpp_intx
fetch lux-cpp/evmmax           refs/tags/v0.21.0 "$LUX_CPP_EVMMAX"           /src/deps/luxcpp_evmmax
fetch lux-cpp/pqclean          refs/tags/v1.0.0  "$LUX_CPP_PQCLEAN"          /src/deps/luxcpp_pqclean
fetch lux-cpp/ed25519-donna    refs/tags/v1.0.0  "$LUX_CPP_ED25519_DONNA"    /src/deps/luxcpp_ed25519_donna
fetch lux-cpp/blake3-reference refs/tags/v1.5.0  "$LUX_CPP_BLAKE3_REFERENCE" /src/deps/luxcpp_blake3_reference
fetch luxfi/c-kzg-4844         refs/tags/v2.1.7  "$LUXFI_C_KZG_4844"         /src/deps/luxfi_c_kzg_4844
fetch luxfi/sr25519-crust      refs/tags/v0.2.0  "$LUXFI_SR25519_CRUST"      /src/deps/sr25519_crust
sealed
EOF

# cevm's dependencies, with no token. The lux packages are on no Conan remote,
# so they are exported from their checkouts first. FetchContent takes each
# dependency from /src/deps (FETCHCONTENT_SOURCE_DIR_<NAME>) instead of cloning
# it, and a dependency not fetched there fails rather than being cloned. blst is built portable, choosing ADX at run time, so the binary does not
# depend on the CPU of the node that happened to build it.
RUN <<'EOF'
set -eu
deps="'FETCHCONTENT_FULLY_DISCONNECTED': {'value': 'ON', 'cache': True, 'type': 'BOOL'}, "
for d in /src/deps/*; do
  n=$(basename "$d" | tr '[:lower:]' '[:upper:]')
  deps="$deps'FETCHCONTENT_SOURCE_DIR_$n': '$d', "
done
conan profile detect --force
printf 'core.download:parallel=1\n' >> "$(conan config home)/global.conf"
conan export /src/luxcpp/crypto
conan export /src/luxcpp/blst
conan export /src/luxcpp/zap-cpp-core
conan install /src/luxcpp/cevm \
  -pr /src/luxcpp/cevm/.github/conan/manylinux-relax.profile \
  -s build_type=Release -s compiler.cppstd=gnu20 \
  -o 'bls/*:portable=True' \
  -c "lux-crypto/*:tools.cmake.cmaketoolchain:extra_variables={$deps}" \
  --output-folder=/src/cevm-conan --build=missing
EOF

# The node and the engine under it. gpu-kernels goes where cevm's CMakeLists
# name it, lux-private/gpu-kernels four levels above lib/evm: it carries the
# 0x100 precompile's 256-bit arithmetic and the aivm oracle.
ARG LUX_CPP_CONSENSUS LUX_GPU_KERNELS LUX_CPP_NODE
RUN --mount=type=secret,id=GH_READ_TOKEN <<'EOF'
set -eu
. /usr/local/lib/fetch.sh
auth
fetch lux-cpp/consensus   refs/heads/main "$LUX_CPP_CONSENSUS" /src/lux-cpp/consensus
fetch lux-gpu/gpu-kernels refs/heads/main "$LUX_GPU_KERNELS"   /src/lux-private/gpu-kernels
fetch lux-cpp/node        refs/heads/main "$LUX_CPP_NODE"      /src/lux-cpp/node
sealed
EOF

# Where lux-cpp/node looks for it: ../../lux/crypto/dist from the top source
# directory, which is this repository at /src/zooai/node.
COPY --from=luxcrypto /src/lux/crypto/dist /src/lux/crypto/dist

COPY CMakeLists.txt /src/zooai/node/
COPY src /src/zooai/node/src
COPY genesis /src/zooai/node/genesis

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
mkdir -p /out/libexec/lux
strip -o /out/libexec/lux/cevm "$(find /src/build -type f -name cevm -perm -u+x | head -n1)"
if ! grep -qF 'zooai/zood/v' "$z"; then
  echo "zood: the binary does not answer as zooai/zood; Zoo's spec did not reach it." >&2
  exit 1
fi
for b in "$z" /out/libexec/lux/cevm; do
  needs=$(readelf -d "$b" | sed -n 's/.*(NEEDED).*\[\(.*\)\]$/\1/p')
  test -n "$needs"
  extra=$(printf '%s\n' "$needs" | grep -Evx 'lib(c|m|pthread|dl|rt|resolv|gcc_s|stdc\+\+|gomp)\.so\.[0-9]+|ld-linux-(x86-64|aarch64)\.so\.[0-9]+' || true)
  if [ -n "$extra" ]; then
    echo "zood: $b links $extra, which gcr.io/distroless/cc-debian12 does not carry." >&2
    exit 1
  fi
done
k=$(mktemp -d)
"$z" --data "$k" --publish > "$k.line"
test -s "$k.line"
rm -rf "$k" "$k.line"
EOF

# ── accept ──────────────────────────────────────────────────────────────────
# Zoo's history through the door a client uses (`--target accept`). A validator
# alone, as an archive runs, imports mainnet's export and then testnet's, and
# four import mainnet's; test/accept.py holds what each serves to the export's
# own hashes and the Go archive's, and a validator alone to refusing
# transactions it could never land.
FROM builder AS accept
ARG LUXFI_STATE=671e4d6328487e7160f08005dac8590d217bb953
RUN --mount=type=secret,id=GH_READ_TOKEN <<'EOF'
set -eu
. /usr/local/lib/fetch.sh
auth
# luxfi/state holds every chain's archive, so only these files' blobs are
# fetched: the commit and its trees, then each blob, checked against the tree.
d=$(mktemp -d)
git init -q "$d"
git -C "$d" remote add origin https://github.com/luxfi/state
git -C "$d" fetch -q --filter=tree:0 origin refs/heads/main
git -C "$d" rev-list FETCH_HEAD | grep -qx "$LUXFI_STATE"
git -C "$d" fetch -q --depth 1 --filter=blob:none origin "$LUXFI_STATE"
for net in mainnet-200200 testnet-200201; do
  path=rlp/zoo-${net%-*}/zoo-$net.rlp
  git -C "$d" cat-file blob "$LUXFI_STATE:$path" > /src/zoo-${net%-*}.rlp
  test "$(git hash-object /src/zoo-${net%-*}.rlp)" = "$(git -C "$d" rev-parse "$LUXFI_STATE:$path")"
done
rm -rf "$d"
sealed
EOF
COPY test/accept.py /usr/local/lib/accept.py
RUN <<'EOF'
set -eu
z=/src/build/zood
vm=/out/libexec/lux/cevm
# The EVM alone first: the plugin replays each export against the genesis zood
# compiles in, so a refusal is named here rather than behind a validator.
"$vm" import /src/zooai/node/genesis/mainnet.json /src/zoo-mainnet.rlp
"$vm" import /src/zooai/node/genesis/testnet.json /src/zoo-testnet.rlp
# alone NETWORK PORT: a validator alone, as a pod with no shell runs it —
# publish, then read the line it left — checked, then stopped.
alone() {
  "$z" --network "$1" --data /tmp/$1 --publish > /dev/null
  "$z" --network "$1" --data /tmp/$1 --committee /tmp/$1/published \
    --peers 127.0.0.1:$(($2 + 1)) --rpc-port "$2" --import-chain-data /src/zoo-$1.rlp \
    --vm "$vm" > /tmp/$1.log 2>&1 &
  pid=$!
  rc=0
  python3 /usr/local/lib/accept.py "$1" 127.0.0.1:"$2" --alone /tmp/$1.log || rc=$?
  kill "$pid" 2>/dev/null || true
  if [ "$rc" -ne 0 ]; then tail -n 30 /tmp/$1.log; exit "$rc"; fi
}
alone mainnet 19620
alone testnet 19610
# Four, a committee that can decide.
: > /tmp/committee
for i in 0 1 2 3; do "$z" --data /tmp/v$i --publish >> /tmp/committee; done
peers=127.0.0.1:19631,127.0.0.1:19641,127.0.0.1:19651,127.0.0.1:19661
for i in 0 1 2 3; do
  "$z" --data /tmp/v$i --committee /tmp/committee --peers "$peers" \
    --rpc-port $((19630 + 10 * i)) --import-chain-data /src/zoo-mainnet.rlp \
    --vm "$vm" > /tmp/v$i.log 2>&1 &
done
rc=0
python3 /usr/local/lib/accept.py mainnet 127.0.0.1:19630 /tmp/v0.log /tmp/v1.log /tmp/v2.log /tmp/v3.log || rc=$?
kill $(jobs -p) 2>/dev/null || true
if [ "$rc" -ne 0 ]; then tail -n 30 /tmp/v*.log; fi
exit "$rc"
EOF

# ── runtime ─────────────────────────────────────────────────────────────────
# cc carries exactly the C and C++ runtimes zood links, and no shell, package
# manager or fetcher: an image that holds a validator's key holds nothing that
# can be told to fetch and run.
FROM gcr.io/distroless/cc-debian12:nonroot@sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f
COPY --from=builder /src/build/zood /usr/local/bin/zood
COPY --from=builder /out/libexec/lux/cevm /usr/local/libexec/lux/cevm
# The JSON-RPC and the validator mesh. A validator is named by the key it proves
# on every link, so the mesh port is not an admin surface and needs no gate.
EXPOSE 9630 9631
USER nonroot
ENTRYPOINT ["/usr/local/bin/zood"]
