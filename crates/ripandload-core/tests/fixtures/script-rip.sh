#!/bin/sh
# Stands in for `rip` in tests, through a symlink named `rip` (see
# tests/common/mod.rs). Runs the test's script from `<symlink>.script`.
# Reading the script with sh, instead of executing a file the test just
# wrote, avoids "Text file busy" when tests run in parallel.
exec sh "$0.script" "$@"
