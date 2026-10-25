#!/bin/sh
# Stands in for `rip` in tests.
# Usage: fake-rip.sh LINES DELAY EXIT_CODE
# Prints LINES numbered lines to stdout, sleeping DELAY seconds before each,
# writes one line to stderr, then exits with EXIT_CODE.
lines=$1
delay=$2
code=$3

i=1
while [ "$i" -le "$lines" ]; do
    sleep "$delay"
    echo "line $i"
    i=$((i + 1))
done
echo "fake-rip: done" >&2
exit "$code"
