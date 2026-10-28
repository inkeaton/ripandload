#!/bin/sh
# Every check a change must pass before it is committed, warnings denied.
#
# Usage: dev/check.sh
#
# Runs fmt, clippy (with the doc lints from Cargo.toml), rustdoc with private
# items, the tests, a comment check for the two rules no lint covers
# (docs/style/comments-rust.md): comment lines end at column 80, and `// ==`
# dividers run exactly to column 80, and a check that docs/internals/ still
# matches the code: its paths, links, anchors and test names. Runs every
# step, then exits 1 if any failed.
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

# Checks docs/internals/ against the code: every backticked path exists,
# every relative link resolves and its #anchor names a heading (GitHub's
# rule: lowercase, punctuation but - and _ dropped, spaces to -, repeats
# numbered -1, -2…), and every test on a "Tested by" line is a `fn` in the
# crates. Code blocks are skipped.
docs() {
    fns=$(mktemp) || return 1
    grep -rhoE 'fn [a-z0-9_]+' crates --include='*.rs' | sort -u >"$fns"
    find docs/internals -name '*.md' -print0 |
        LC_ALL=C.UTF-8 xargs -0 awk -v fns="$fns" '
            function anchor(heading) {
                heading = tolower(heading)
                gsub(/[^[:alnum:] _-]/, "", heading)
                gsub(/ /, "-", heading)
                return heading
            }
            # Records the anchors of the headings in `file`, once.
            function load(file,    line, fence, a, n) {
                if (file in loaded) return
                loaded[file] = 1
                while ((getline line < file) > 0) {
                    if (line ~ /^```/) { fence = !fence; continue }
                    if (fence || line !~ /^#+ /) continue
                    sub(/^#+ /, "", line)
                    a = anchor(line)
                    n = repeats[file, a]++
                    anchors[file, n ? a "-" n : a] = 1
                }
                close(file)
            }
            function exists(path) {
                return system("test -e \"" path "\"") == 0
            }
            function fail(message) {
                print FILENAME ":" FNR ": " message; bad = 1
            }
            BEGIN {
                while ((getline name < fns) > 0) {
                    sub(/^fn /, "", name)
                    is_fn[name] = 1
                }
            }
            FNR == 1 { fence = 0; tested = 0 }
            /^```/ { fence = !fence; next }
            fence { next }
            {
                rest = $0
                while (match(rest, /`(crates|tests|dev|docs)\/[^` ]*`/)) {
                    path = substr(rest, RSTART + 1, RLENGTH - 2)
                    rest = substr(rest, RSTART + RLENGTH)
                    if (!exists(path)) fail("no such path: " path)
                }

                rest = $0
                while (match(rest, /\]\([^)]*\)/)) {
                    target = substr(rest, RSTART + 2, RLENGTH - 3)
                    rest = substr(rest, RSTART + RLENGTH)
                    if (target ~ /^[a-z]+:/) continue
                    file = target
                    fragment = ""
                    if ((hash = index(target, "#")) > 0) {
                        file = substr(target, 1, hash - 1)
                        fragment = substr(target, hash + 1)
                    }
                    dir = FILENAME
                    sub(/\/[^\/]*$/, "", dir)
                    file = file == "" ? FILENAME : dir "/" file
                    if (!exists(file)) {
                        fail("broken link: " target)
                        continue
                    }
                    if (fragment == "") continue
                    load(file)
                    if (!((file, fragment) in anchors))
                        fail("no heading for: " target)
                }

                # A "Tested by" paragraph runs to the next blank line.
                if ($0 ~ /^Tested by:/) tested = 1
                else if ($0 == "") tested = 0
                rest = $0
                while (tested && match(rest, /`[a-z0-9_]+`/)) {
                    name = substr(rest, RSTART + 1, RLENGTH - 2)
                    rest = substr(rest, RSTART + RLENGTH)
                    if (!(name in is_fn)) fail("no test named: " name)
                }
            }
            END { exit bad }
        '
    status=$?
    rm -f "$fns"
    return $status
}

step fmt cargo fmt --all --check
step clippy cargo clippy --workspace --all-targets -- -D warnings
step doc env RUSTDOCFLAGS="-D warnings" \
    cargo doc --workspace --no-deps --document-private-items
step test cargo test --workspace
step comments comments
step docs docs

if [ -n "$failed" ]; then
    echo "Failed:$failed" >&2
    exit 1
fi
echo "All checks passed."
