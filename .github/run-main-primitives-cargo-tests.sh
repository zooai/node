#!/bin/bash

# export INSTALL_FOLDER_PATH=${INSTALL_FOLDER_PATH:-"/app/pre-install"}
cd /app/zoo-libs/zoo-message-primitives && cargo test -- --test-threads=1 --nocapture

