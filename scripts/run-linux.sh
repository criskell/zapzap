#!/bin/sh
# Runs the core and the ui joined by two pipes: conversations flow core -> ui, what you type
# flows ui -> core. Closing the window stops the core.
#   scripts/run-linux.sh          connect to WhatsApp (zapzap-engine: pairing QR, messages, call rings)
#   scripts/run-linux.sh --demo   play stand-in conversations, no connection (zapzap-core)
root=$(cd "$(dirname "$0")/.." && pwd)
ui="$root/target/x86_64-unknown-none/release/zapzap-ui"
if [ "$1" = "--demo" ]; then
    core="$root/target/x86_64-unknown-linux-musl/release/zapzap-core"
    [ -x "$core" ] || core="$root/target/release/zapzap-core"
else
    core="$root/target/x86_64-unknown-linux-gnu/release/zapzap-engine"
    [ -x "$core" ] || core="$root/target/release/zapzap-engine"
    [ "$1" = "--live" ] && shift
fi

dir=$(mktemp -d) && mkfifo "$dir/to-ui" "$dir/to-core" && trap 'rm -rf "$dir"' EXIT
# Both sides open the pipes in the same order, so neither waits on the other forever.
"$core" "$@" >"$dir/to-ui" <"$dir/to-core" &
core_pid=$!
"$ui" <"$dir/to-ui" >"$dir/to-core"
kill "$core_pid" 2>/dev/null
