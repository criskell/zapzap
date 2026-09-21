#!/bin/sh
# Smallest Linux build: no_std, no libc, no allocator, raw syscalls (x86_64 only). Needs nightly + rust-src.
# Output: target/x86_64-unknown-none/release/zapzap-ui
exec env RUSTFLAGS="-C code-model=small -C relocation-model=static" \
    cargo build --release -p zapzap-ui --target x86_64-unknown-none \
    -Z build-std=core,compiler_builtins -Z build-std-features=compiler-builtins-mem
