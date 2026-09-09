# zood — the Zoo network node.
#
# Built ON the node it links. lux-cpp/node publishes its builder stage beside
# its runtime, carrying libnode.a, the headers, and the consensus, cevm and
# aws-lc trees they were compiled against. Building on that is what makes this
# the same node every peer runs; assembling those three checkouts again here
# would be a second build of the thing they all have to match.
ARG HOST=ghcr.io/lux-cpp/node:main-dev
FROM ${HOST} AS build

WORKDIR /zoo
COPY CMakeLists.txt ./
COPY src src
COPY test test

# The host's own build left its dependency locations in place, so this resolves
# them rather than restating them.
RUN cmake -S . -B build -G Ninja \
      -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_EXE_LINKER_FLAGS="-s" \
      -DLUX_NODE_DIR=/src/lux-cpp/node \
      -DCONSENSUS_DIR=/src/lux-cpp/consensus \
      -DCEVM_DIR=/src/luxcpp/cevm \
      -DLUXCPP_ROOT=/src/luxcpp \
      -DZAP_DIR=/src/luxcpp/zap-cpp-core/include \
      -DAWSLC_SRC=/src/aws-lc \
      -DAWSLC_BUILD_DIR=/src/aws-lc-build \
 && cmake --build build --target zood \
 && test -s build/zood

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /zoo/build/zood /usr/local/bin/zood
# The JSON-RPC and the validator mesh. A validator is named by the key it proves
# on every link, so the mesh port is not an admin surface and needs no gate.
EXPOSE 9630 9631
USER nonroot
ENTRYPOINT ["/usr/local/bin/zood"]
