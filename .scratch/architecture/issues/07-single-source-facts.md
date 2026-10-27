# 07 – Facts written in two places

Type: grilling
Status: open
Blocked by: none

Part of `../map.md`. Candidate 6 of the review (Worth exploring).

## Question

For each fact below that is written twice and kept in step by hand, should it be computed or referenced from one place, and how?

1. **Shared Quality choices.** `quality_choices` (`source.rs`) hard-codes `(1, "MP3 320 kbps"), (2, "CD quality (16-bit FLAC)")` for a mix of Qobuz and Deezer, instead of intersecting `quality_levels`. Qobuz and Deezer label the shared levels differently, so the intersection needs a rule for the label. A third Source would silently get the hard-coded answer.
2. **Managed keys.** The `streamrip_config.rs` header lists the keys the app manages; `apply` writes them with 12 `set_managed` calls; the tests check them again. A `MANAGED_KEYS` table could drive all three. `set_managed` is `pub` but only used inside the module (smell absorbed: make it private).
3. **The WSL SSH key path.** `COPY_SSH_KEY` (`tool_env.rs`) spells `~/.ssh/ripandload_key` twice instead of using `WSL_SSH_KEY`; tests spell it three more times. Options: build the script with `concat!` from a shared literal macro, or pass the destination as `$2` (using `$HOME`, since `~` doesn't expand inside quotes).

## Resolution produces

A decision per fact (do it, or leave it with the reason) and implementation tickets in `../fixes/issues/`, blocked by fix 03 and 10 where they touch the same files.
