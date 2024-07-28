#!/bin/bash
CROSS_TARGET=x86_64-unknown-linux-gnu

PACKAGES="user"
for package in $PACKAGES; do
    cargo zigbuild --target=$CROSS_TARGET --package $package --release
done


    # Run cargo-deb to build Debian package
for package in $PACKAGES; do
    cargo deb --no-build --no-strip --target=$CROSS_TARGET -p $package || { echo "Error: Failed to build Debian package for $CRATE_DIR"; exit 1; }
done




