#!/bin/sh
# Resolve script directory and change to it
cd "$(dirname "$0")"

echo "Building Debian package..."

# Create staging directory structure
mkdir -p debian/usr/bin
mkdir -p ../../dist/packages

# Locate and copy binary
if [ -f "../../dist/binaries/ridle" ]; then
    cp ../../dist/binaries/ridle debian/usr/bin/ridle
elif [ -f "../../target/x86_64-unknown-linux-musl/release/ridle" ]; then
    cp ../../target/x86_64-unknown-linux-musl/release/ridle debian/usr/bin/ridle
elif [ -f "../../target/release/ridle" ]; then
    cp ../../target/release/ridle debian/usr/bin/ridle
else
    echo "Error: compiled ridle binary not found in target/ or dist/binaries/."
    exit 1
fi

chmod 755 debian/usr/bin/ridle

# Run dpkg-deb to build the package
dpkg-deb --build debian ../../dist/packages/ridle.deb

# Clean up staging binary
rm -f debian/usr/bin/ridle
