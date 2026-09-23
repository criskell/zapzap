#!/bin/sh
# Puts the trimmed whatsapp-rust the workspace builds against next to the repository (../whatsapp-rust):
# upstream v0.7.0 with patches/whatsapp-rust-0.7.0.patch on top. Safe to run again.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
fork="$root/../whatsapp-rust"
if [ ! -d "$fork/.git" ]; then
    git clone --quiet --depth 1 --branch v0.7.0 https://github.com/jlucaso1/whatsapp-rust "$fork"
fi
cd "$fork"
if git apply --check "$root/patches/whatsapp-rust-0.7.0.patch" 2>/dev/null; then
    git apply "$root/patches/whatsapp-rust-0.7.0.patch"
    echo "whatsapp-rust: patch applied in $fork"
else
    echo "whatsapp-rust: already patched (or the tree differs) in $fork"
fi
