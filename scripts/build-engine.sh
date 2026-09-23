#!/bin/sh
# Smallest engine build (nightly): no unwind tables, no PIE relocations, no Debug formatting or panic
# locations, SQLite without full-text search, JSON, R-tree and the other extras ZapZap never uses. Output: target/x86_64-unknown-linux-gnu/release/zapzap-engine
# `--target` keeps these flags off build scripts and proc macros, which must stay position independent.
exec env \
    RUSTFLAGS="-C force-unwind-tables=no -C relocation-model=static -C link-arg=-no-pie -Z fmt-debug=none -Z location-detail=none" \
    LIBSQLITE3_FLAGS="-USQLITE_ENABLE_FTS3 -USQLITE_ENABLE_FTS3_PARENTHESIS -USQLITE_ENABLE_FTS5 -USQLITE_ENABLE_JSON1 -USQLITE_ENABLE_RTREE -USQLITE_ENABLE_STAT4 -USQLITE_SOUNDEX -USQLITE_ENABLE_DBSTAT_VTAB -USQLITE_ENABLE_COLUMN_METADATA -USQLITE_ENABLE_LOAD_EXTENSION -USQLITE_ENABLE_MEMORY_MANAGEMENT -USQLITE_ENABLE_API_ARMOR -DSQLITE_OMIT_LOAD_EXTENSION -DSQLITE_OMIT_DEPRECATED -DSQLITE_OMIT_SHARED_CACHE -DSQLITE_OMIT_PROGRESS_CALLBACK -DSQLITE_OMIT_JSON -DSQLITE_OMIT_TRACE -DSQLITE_DEFAULT_MEMSTATUS=0 -DSQLITE_DQS=0" \
    CFLAGS="-fno-unwind-tables -fno-asynchronous-unwind-tables" \
    cargo build --release -p zapzap-engine --target x86_64-unknown-linux-gnu \
    -Z build-std=std,panic_abort -Z build-std-features=optimize_for_size "$@"
