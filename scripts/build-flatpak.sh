#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.." 

# Update lockfile and regenerate vendor
rm -rf vendor vendor.tar.gz
cargo generate-lockfile
echo "Generating vendor directory..."
cargo vendor vendor
echo "Creating vendor.tar.gz..."
tar czf vendor.tar.gz vendor
echo "Vendor ready!"

# Build Flatpak
echo "Building Flatpak..."
flatpak-builder --force-clean --repo=/tmp/repo build-dir com.github.toasterrepair.Stickerbook.json

# Create bundle
echo "Creating bundle..."
flatpak build-bundle /tmp/repo stickerbook.flatpak com.github.toasterrepair.Stickerbook

echo "Done! Bundle created: stickerbook.flatpak"

# Cleanup prompt
echo ""
read -p "Remove build directory, repo, and flatpak-builder cache? [y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    rm -rf build-dir /tmp/repo .flatpak-builder
    echo "Cleaned up build artifacts."
fi
