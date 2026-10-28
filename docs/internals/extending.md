# Extending the app

This is for someone about to change the app: add a Setting, a Source, or a
new kind of Run. Each walkthrough lists the files to touch, in the order the
data flows through them, with one sentence per step and a link to the
pipeline that explains that code. Read [architecture.md](architecture.md)
first.

Source, Settings, Run and the other capitalised terms are defined in the
[glossary](../../GLOSSARY.md).

**Let the compiler help, but not lead.** After the first step of a
walkthrough, the compiler points at most of the places that need a new
case: every `match` must cover every variant, so a new one is an error
until each is handled (→ [primer: `match`](rust-primer.md#match)). It won't
point at everything. A key written into the Streamrip config, a doc comment
that lists the cases, a test that uses the old behaviour as its example:
those compile either way. The steps below include them.

**Rehearsed.** Each walkthrough was followed on a throwaway branch, step by
step, until `dev/check.sh` passed, and its diff touched exactly the files
listed. The walkthrough for a new Setting was rehearsed against commit
`d522556`. If the code has moved on since, the steps may need adjusting.

## A new Setting: streamrip's `max_connections`

The example: let the user choose how many tracks streamrip downloads at
once. streamrip 2.2.0 reads it from `[downloads] max_connections` in its
config. Its default is 6, and 0 or less means no limit.

The app makes it a `u8` from 1 to 16, defaulting to 6, and doesn't offer
"no limit": a friend on a slow line shouldn't be able to choke it by
accident.

1. **`crates/ripandload-core/src/settings.rs`, `Settings`.** Add the field,
   with its doc comment, and its value in `Settings::default`, which
   `#[serde(default)]` also uses when an older Settings file lacks the key.
   Add a test that such a file still loads, next to
   `settings_from_before_the_clear_toggle_still_offer_to_clear`:

   ```rust
   /// A file from before `max_connections` loads streamrip's default, 6.
   #[test]
   fn settings_from_before_max_connections_load_streamrips_default() {
       let (_dir, path) = temp_path();
       fs::create_dir_all(path.parent().unwrap()).unwrap();
       fs::write(&path, "staging_folder = \"/music\"\n").unwrap();

       assert_eq!(Settings::load(&path).unwrap().max_connections, 6);
   }
   ```

   → [Settings: loading](pipelines/settings.md#1-loading)

2. **`settings.rs`, `Settings::validate`.** Add a `MAX_CONNECTIONS` constant,
   a `SettingsProblem::MaxConnectionsOutOfRange(u8)` variant with its
   `#[error(...)]` message, the range check, and a line for it in
   `validate`'s `# Errors` list. Test 0 and 17, as the Quality range test
   does. The problem shows on the Settings tab by itself.
   → [Settings: problems](pipelines/settings.md#3-problems)

3. **`crates/ripandload-core/src/streamrip_config.rs`, `apply`.** Write the
   key on every Download:

   ```rust
   set_managed(
       doc,
       "downloads",
       "max_connections",
       i64::from(settings.max_connections),
   );
   ```

   TOML integers are `i64`, hence the conversion. Add the key to the
   module doc's `# Managed keys` list, which nothing checks.
   → [Settings: the managed keys](pipelines/settings.md#7-the-managed-keys)

4. **`crates/ripandload-core/tests/streamrip_config.rs`.** Three changes,
   none flagged by the compiler:
   - the `settings` helper sets every managed key to a value that differs
     from the personal config fixture's, so set `max_connections` to 3
     there (the fixture has 6, the default);
   - `seeded_from_personal_config_with_managed_keys_from_settings` checks
     every managed key, so add an assertion for this one;
   - `unmanaged_keys_and_comments_survive` and
     `existing_config_is_updated_not_reseeded` use `max_connections` as
     their example of a key the app *doesn't* manage. They fail once it
     is. Switch them to another unmanaged key in the fixture, such as
     `[filepaths] truncate_to`.

   → [Settings: rendering](pipelines/settings.md#6-rendering)

5. **`crates/ripandload/src/settings_tab.rs`, `folders_and_tools`.** Add a
   row with a `DragValue` limited to `1..=MAX_CONNECTIONS`, and add the row
   to the function's doc comment, which lists them. Unsaved changes and
   Save need nothing: they compare whole `Settings` with `PartialEq`, so
   the new field is included.
   → [Settings: editing](pipelines/settings.md#2-editing) and
   [unsaved changes](pipelines/settings.md#4-unsaved-changes)

No glossary entry is needed: the glossary names concepts, not each Setting.

The rehearsal's diff touched exactly the four files above: `settings.rs`,
`streamrip_config.rs`, the integration test `streamrip_config.rs`, and
`settings_tab.rs`. These documents are
outside that diff. [Settings round-trip](pipelines/settings.md) uses
`max_connections` as its example of a key the user added by hand. A real
change would give that example another key.

## Before you commit

- Run `dev/check.sh`. It runs fmt, clippy with the doc lints, rustdoc, the
  tests and the comment check, with warnings denied.
- Read [`docs/style/comments.md`](../style/comments.md) and
  [`docs/style/comments-rust.md`](../style/comments-rust.md) before
  writing or rewriting comments.
- Ask whether the change is a decision worth recording: hard to reverse,
  surprising without context, and the result of a real trade-off. If so,
  add an ADR to [`docs/adr/`](../adr/).
