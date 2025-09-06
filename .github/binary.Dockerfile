FROM ubuntu:24.10 AS downloader
 RUN apt-get update && apt-get install -y curl unzip
 ARG ZOO_NODE_VERSION
 RUN curl -L -o zoo-node.zip https://download.zoo.ngo/zoo-node/binaries/production/x86_64-unknown-linux-gnu/${ZOO_NODE_VERSION:-latest}.zip
 RUN FILE_SIZE=$(stat -c %s /zoo-node.zip) && \
    if [ $FILE_SIZE -lt 26214400 ]; then \
        echo "Error: zoo-node file is less than 25MB" && \
        exit 1; \
    fi
 RUN unzip -o zoo-node.zip -d ./node
 RUN chmod +x /node/zoo-node

 FROM ubuntu:24.10 AS runner
 RUN apt-get update && apt-get install -y openssl ca-certificates
 WORKDIR /app
 COPY --from=downloader /node ./

 ENV ZOO_TOOLS_RUNNER_DENO_BINARY_PATH="/app/zoo-tools-runner-resources/deno"
 ENV ZOO_TOOLS_RUNNER_UV_BINARY_PATH="/app/zoo-tools-runner-resources/uv"
 ENV PATH="/app/zoo-tools-runner-resources:/root/.local/bin:$PATH"

 EXPOSE 9550
 ENTRYPOINT ["/bin/sh", "-c", "/app/zoo-node"]