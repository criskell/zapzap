#!/bin/sh
# Usage: scripts/mem.sh <pid>
# Prints RSS, anonymous RSS and PSS (kB) of a running process. Linux only.
pid="${1:?pid}"
grep -E '^(VmRSS|RssAnon|RssFile)' "/proc/$pid/status"
awk '/^Pss:/ { s += $2 } END { print "Pss:", s, "kB" }' "/proc/$pid/smaps"
