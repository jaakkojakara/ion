#!/bin/bash

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Change to project root directory
cd "$PROJECT_ROOT"

# Build WASM
RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals -A unreachable-code -A unused-variables -A unused-mut -A unused_imports" wasm-pack build ion_game --out-dir "../target/web/pkg" --target no-modules -Z build-std=panic_abort,std

# Copy web files
cp "ion_engine/src_web/index.html" "target/web/index.html"
cp "ion_engine/src_web/worker.js" "target/web/worker.js"
cp "ion_engine/src_web/server.js" "target/web/server.js"

# Asset copying with change detection
ASSETS_DIR="ion_game/assets"
TARGET_DIR="target/web/assets"
HASH_FILE="target/web/.assets_hash"

calculate_hash() {
    # Find all files in assets directory, sort them, and create a hash of their contents and modification times
    find "$ASSETS_DIR" -type f -exec sha256sum {} \; | sort | sha256sum | cut -d' ' -f1
}

mkdir -p "$TARGET_DIR"

current_hash=$(calculate_hash)

# Check if we need to copy files
if [ -f "$HASH_FILE" ]; then
    stored_hash=$(cat "$HASH_FILE")
    if [ "$current_hash" = "$stored_hash" ]; then
        echo "Assets unchanged, skipping copy"
    else
        echo "Assets changed, copying files..."
        rm -rf "$TARGET_DIR"/*
        cp -r "$ASSETS_DIR"/* "$TARGET_DIR"/
        echo "$current_hash" > "$HASH_FILE"
        echo "Assets copied successfully"
    fi
else
    echo "First time copying assets..."
    cp -r "$ASSETS_DIR"/* "$TARGET_DIR"/
    echo "$current_hash" > "$HASH_FILE"
    echo "Assets copied successfully"
fi

