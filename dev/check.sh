#!/bin/sh
# Every check a change must pass before it is committed, warnings denied.
#
# Usage: dev/check.sh
#
# Runs fmt, clippy (with the doc lints from Cargo.toml), rustdoc with private
# items, the tests, and a comment check for the two rules no lint covers
# (docs/style/comments-rust.md): comment lines end at column 80, and `// ==`
# dividers run exactly to column 80. Runs every step, then exits 1 if any
# failed.
set -u

cd "$(dirname "$0")/.." || exit 1
failed=""

# Runs the command after the step name, recording the name if it fails.
step() {
    name=$1
    shift
    echo "== $name"
    "$@" || failed="$failed $name"
}

# Only whole-line comments: a comment after code follows rustfmt's width.
comments() {
    find crates -name '*.rs' -not -path '*/target/*' -print0 |
        LC_ALL=C.UTF-8 xargs -0 awk '
            /^[[:space:]]*\/\// && length($0) > 80 {
                print FILENAME ":" FNR ": comment past column 80"; bad = 1
            }
            /^[[:space:]]*\/\/ ==/ && (length($0) != 80 || $0 !~ /\/\/ == [^= ].* =+$/) {
                print FILENAME ":" FNR \
                    ": divider not `// == Title ===` to column 80"
                bad = 1
            }
            END { exit bad }
        '
}

step fmt cargo fmt --all --check
step clippy cargo clippy --workspace --all-targets -- -D warnings
step doc env RUSTDOCFLAGS="-D warnings" \
    cargo doc --workspace --no-deps --document-private-items
step test cargo test --workspace
step comments comments

if [ -n "$failed" ]; then
    echo "Failed:$failed" >&2
    exit 1
fi
echo "All checks passed."
