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
`d522556`, and the one for a new Source against `db56b2a`. If the code has
moved on since, the steps may need adjusting.

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

## A new Source: Tidal

The example: let the user paste Tidal URLs. streamrip 2.2.0 downloads from
Tidal, and its URLs look like `https://tidal.com/browse/album/251380836`
or `https://listen.tidal.com/track/251380837`.

**Tidal has no Credentials to paste.** Qobuz and Deezer give the user a
token or a cookie to copy into the Settings. Tidal doesn't. streamrip's
`[tidal]` section holds `user_id`, `country_code`, `access_token`,
`refresh_token` and `token_expiry`, all marked "Do not change". `rip` fills
them in itself: with no access token, it prints a link, opens the browser,
and waits up to five minutes for the user to log in there. It then
refreshes the access token every week.

So the app must leave those keys alone. If `apply` managed them, every
Download would overwrite streamrip's fresh tokens with stale ones. The
walkthrough manages only Tidal's Quality, and lets the first Tidal Download
log in. That needs nothing from the app: `rip` reads no stdin for it, and
its line `Go to https://link.tidal.com/… to log into Tidal within 5
minutes.` lands in the Log like any other.

The Preview reads the access token from the Streamrip config, the way Qobuz
Previews read the `app_id`. Until the first Tidal Download, a Tidal URL
says `no preview until the first Tidal Download`.

1. **`crates/ripandload-core/src/source.rs`, `Source`.** Add `Tidal`, and
   rewrite the doc comment that says Tidal isn't a Source. Recognise its
   hosts in `from_url` (`tidal.com` and its subdomains), give it
   `quality_levels` `0..=3`, and name it in `Display`. Test its URLs and the
   Quality levels it shares with Qobuz. Also fix `other_sources_are_none`,
   which uses a Tidal URL as its example of *no* Source.

   Check that `quality_label` still reads right: it names levels 0 and 1
   "MP3", and Tidal's are AAC. The rehearsal left the labels as they are.
   → [URL recognition: recognising the Source](pipelines/url-recognition.md#1-recognising-the-source)

2. **`crates/ripandload-core/src/preview.rs` and `error.rs`.** Read the URL
   in `parse`:

   ```rust
   Source::Tidal => match segments[..] {
       ["browse", kind, id] | [kind, id] => (kind, id),
       _ => return None,
   },
   ```

   Add a `TidalAccess` struct (the access token and the country code) and
   a `TIDAL_API` base. `fetch` takes a `&TidalAccess` next to the
   `&QobuzAccess`, and its Tidal arm asks for `albums/<id>` with
   `countryCode` and an `Authorization: Bearer` header. With no access
   token it returns a new `PreviewError::NoTidalLogin`, "no preview until
   the first Tidal Download". Add `describe` arms: Tidal's album and track
   answers keep the names where Deezer's do. Update the
   `PreviewError::Unsupported` doc, which names the Sources. Test the URL
   shapes and `describe`, with fixtures `tidal-album.json` and
   `tidal-track.json` in `crates/ripandload-core/tests/fixtures/preview/`.
   The rehearsal wrote them by hand from the API's shape. Record real ones
   if you can. Also fix `other_urls_have_no_link`, which lists a Tidal URL
   as "not a Source at all".

   Tidal playlist IDs hold hyphens, which `parse` refuses, so playlists get
   no Preview.
   → [URL recognition: reading the URL](pipelines/url-recognition.md#4-reading-the-url),
   [asking the API](pipelines/url-recognition.md#5-asking-the-api) and
   [naming it](pipelines/url-recognition.md#6-naming-it)

3. **`crates/ripandload-core/src/streamrip_config.rs`, `tidal_access`.** A
   reader next to `qobuz_app_id`, giving the `[tidal]` access token and
   country code, empty when they're missing.
   → [URL recognition: starting a lookup thread](pipelines/url-recognition.md#3-starting-a-lookup-thread)

4. **`crates/ripandload/src/download_tab.rs`, `Previews`.** Cache a
   `TidalAccess` per round, as `qobuz_access` is. Read it through a
   `read_tidal_access` field, so tests can replace it, clear it in `retry`,
   and pass it to `fetch`. Add `NoTidalLogin` to the errors `state_of`
   shows as they are, and to its test. Change the two texts that name the
   Sources: the label above the text box, and the
   `⚠ not a Qobuz or Deezer URL` line.
   → [URL recognition: before the first Qobuz Download](pipelines/url-recognition.md#example-d-before-the-first-qobuz-download)

5. **`crates/ripandload-core/src/settings.rs`, `QualityDefaults`.** Add
   `tidal`, defaulting to 3, streamrip's own default. Add
   `SettingsProblem::TidalQualityOutOfRange`, its check in `validate`, and
   its line in the Quality range test. Test that an older Settings file's
   `[quality]` table, without `tidal`, still loads: `#[serde(default)]` on
   `QualityDefaults` fills it in. No Credentials struct: there are none.
   → [a new Setting](#a-new-setting-streamrips-max_connections), steps 1
   and 2

6. **`streamrip_config.rs`, `apply`.** One `set_managed` line, for
   `[tidal] quality` and nothing else, and its line in the module doc's
   `# Managed keys` list. In `crates/ripandload-core/tests/streamrip_config.rs`,
   set the `settings` helper's Tidal Quality to 1 (the fixture has 3),
   assert it in the seeding test, and move
   `unmanaged_keys_and_comments_survive` off `[tidal] quality`, its example
   of an unmanaged key, to one such as `[database] downloads_enabled`.
   → [a new Setting](#a-new-setting-streamrips-max_connections), steps 3
   and 4

7. **`crates/ripandload/src/settings_tab.rs`.** A Quality row,
   `level_picker(ui, Source::Tidal, &mut settings.quality.tidal)`, and a
   row under Credentials saying that streamrip asks the user to log in on
   the first Tidal Download.
   → [a new Setting](#a-new-setting-streamrips-max_connections), step 5

8. **Docs.** The user README names the Sources four times, and explains
   Credentials. The glossary's Source entry already lists Tidal. Its
   Credentials entry gets a sentence saying Tidal has none in the Settings.

The rehearsal's diff touched exactly these files: `source.rs`, `preview.rs`,
`error.rs`, `streamrip_config.rs` and the integration test of the same
name, `settings.rs`, `download_tab.rs`, `settings_tab.rs`, `README.md` and
`GLOSSARY.md`, plus the two new fixtures.

The access token lasts a week, and streamrip refreshes it while a Download
runs. A Preview made with an out-of-date token fails, and says
`no preview`, like any other failed lookup.

## Before you commit

- Run `dev/check.sh`. It runs fmt, clippy with the doc lints, rustdoc, the
  tests and the comment check, with warnings denied.
- Read [`docs/style/comments.md`](../style/comments.md) and
  [`docs/style/comments-rust.md`](../style/comments-rust.md) before
  writing or rewriting comments.
- Ask whether the change is a decision worth recording: hard to reverse,
  surprising without context, and the result of a real trade-off. If so,
  add an ADR to [`docs/adr/`](../adr/).
