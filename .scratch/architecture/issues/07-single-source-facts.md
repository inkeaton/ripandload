# 07 – Facts written in two places

Type: grilling
Status: resolved
Blocked by: none

Part of `../map.md`. Candidate 6 of the review (Worth exploring).

## Question

For each fact below that is written twice and kept in step by hand, should it be computed or referenced from one place, and how?

1. **Shared Quality choices.** `quality_choices` (`source.rs`) hard-codes `(1, "MP3 320 kbps"), (2, "CD quality (16-bit FLAC)")` for a mix of Qobuz and Deezer, instead of intersecting `quality_levels`. Qobuz and Deezer label the shared levels differently, so the intersection needs a rule for the label. A third Source would silently get the hard-coded answer.
2. **Managed keys.** The `streamrip_config.rs` header lists the keys the app manages; `apply` writes them with 12 `set_managed` calls; the tests check them again. A `MANAGED_KEYS` table could drive all three. `set_managed` is `pub` but only used inside the module (smell absorbed: make it private).
3. **The WSL SSH key path.** `COPY_SSH_KEY` (`tool_env.rs`) spells `~/.ssh/ripandload_key` twice instead of using `WSL_SSH_KEY`; tests spell it three more times. Options: build the script with `concat!` from a shared literal macro, or pass the destination as `$2` (using `$HOME`, since `~` doesn't expand inside quotes).

## Resolution produces

A decision per fact (do it, or leave it with the reason) and implementation tickets in `../fixes/issues/`, blocked by fix 03 and 10 where they touch the same files.

## Answer

Grilled 2026-10-27.

1. **Shared Quality choices: yes.** streamrip numbers levels the same for every Source (0 MP3 128 … 4 Hi-Res >96); only the labels differed. Labels become per level (`quality_label(level)`), and `quality_choices` is the intersection of the Sources' ranges. Deezer's 2 now reads "CD (16-bit / 44.1 kHz)" rather than "FLAC". A third Source needs no extra code.
2. **Managed keys: no table.** A `MANAGED_KEYS` table needs a closure per row for four value types and reads worse than 12 calls. The tests restating the values are the tests' oracle, not drift, and the module header's prose says where each value comes from, which a table wouldn't. Only `set_managed` becomes private.
3. **WSL SSH key path: yes, with `concat!`** from one `macro_rules!` literal. Passing the destination as `$2` was rejected: it changes the script's quoting and `ssh -i` would get `$HOME/…` unexpanded.

Implementation: `../../fixes/issues/19-facts-written-once.md` (blocked by 18). No ADR: none of the three is hard to reverse.
