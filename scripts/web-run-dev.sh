#!/bin/bash

# Exit on any error
set -e

# Run the build script first
./scripts/web-build.sh

# Start the dev server within the target/web directory
cd target/web
node server.js
