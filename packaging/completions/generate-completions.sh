#!/bin/sh
# Resolve script directory and change to it
cd "$(dirname "$0")"

echo "Generating shell completion scripts..."
# Typically: ../../target/release/ridle --generate-completions <shell>
