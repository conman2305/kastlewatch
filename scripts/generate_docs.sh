#!/bin/bash
set -e

# Generate CRD YAML
echo "Generating CRD YAML..."
cargo run -- crdgen > crds.yaml

# Check if crdoc is installed
if ! command -v crdoc &> /dev/null; then
    echo "crdoc not found. Please install it or use a docker container."
    echo "See https://github.com/fybrik/crdoc"
    exit 1
fi

# Generate Documentation
echo "Generating Documentation..."
mkdir -p docs
crdoc --resources crds.yaml --output docs/api-reference.md

echo "Done! Documentation generated at docs/api-reference.md"
