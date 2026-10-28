# Rust primer

This covers only the Rust this codebase uses, each feature shown with a
real excerpt from it. It's a reference, not a course: the other documents
link here from a `> **Rust:**` note the first time a feature matters, and
each entry ends with where those notes are. For everything else, read
[*The Rust Programming Language*](https://doc.rust-lang.org/book/).

The parts build on each other, so an entry relies only on the ones above
it.

Terms such as Run, Source and Settings are defined in the
[glossary](../../GLOSSARY.md).

## Part 1: Reading a file

### Crates, modules, `use` and visibility

A *crate* is a library or a program, the unit Cargo builds. This workspace
has two. A crate is split into *modules*, usually one per file, declared by
its parent. Everything is private to its module unless marked `pub`.
`pub(crate)` is in between: visible across the crate, not outside it.

`crates/ripandload-core/src/lib.rs` declares the core crate's modules:

```rust
pub mod command;
pub mod error;
pub mod live_output;
// …
pub mod wsl_path;
```

`crates/ripandload-core/src/run.rs` has submodules of its own, private,
and re-exports one item from each for the rest of the crate:

```rust
mod check_tools;
// …
pub(crate) use check_tools::CheckTools;
```

`use` brings a name into scope, like an import in Java or Python. `crate::`
starts a path at the crate's root, `super::` at the parent module.

Seen in: [architecture: module map](architecture.md#module-map).

### Doc comments and attributes

`///` is a doc comment for the item below it. `//!`, at the top of a file,
documents the module itself. rustdoc turns both into the API docs.

`#[...]` is an *attribute*: an instruction to the compiler about the item
below. `Source` in `crates/ripandload-core/src/source.rs`:

```rust
/// A Source the app can download from.
///
/// Only the Sources the app has Credentials for, so Tidal and SoundCloud
/// aren't among them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// Qobuz, reached with a user ID and auth token.
    Qobuz,
```

`#[derive(...)]` is the most common attribute here (see
[derive](#derive-the-debug-form-and-serde)). Others say when to compile
something (`#[cfg(...)]`, see [`cfg`](#cfg)) or mark a test (`#[test]`).

### Macros at a glance

A name ending in `!` is a *macro*: code that writes code at compile time.
The ones used everywhere:

- `format!` builds a `String`, with `{}` holes, as `printf` would.
- `vec!` builds a `Vec`, a growable list.
- `matches!` says whether a value fits a pattern, as a `bool`.
- `assert!` and `assert_eq!` fail a test.

`CheckTools::on_line` in `crates/ripandload-core/src/run/check_tools.rs`:

```rust
return vec![Note::Line(format!(
    "→ found streamrip at {line}; to use it, put that path in Settings"
))];
```

`{line}` inside the string names a variable directly.

The codebase writes one macro of its own, with `macro_rules!`: `wsl_ssh_key`
in `tool_env.rs`, which expands to one string literal, so that two
constants can be built from it with `concat!`.

### Blocks as expressions, and `return`

In Rust, a block `{ … }` has a value: its last expression, written without
a `;`. So does an `if`. `check_trailing_slash` in
`crates/ripandload-core/src/command.rs` returns the value of its `if`:

```rust
fn check_trailing_slash(value: &str) -> String {
    if value.ends_with("/") {
        value.to_string()
    } else {
        format!("{value}/")
    }
}
```

`return` says the same thing explicitly. `Upload::name` in
`crates/ripandload-core/src/run/upload.rs`:

```rust
fn name(&self) -> &'static str {
    return "Upload";
}
```

Both mean the same. This codebase mostly writes `return`, for readability;
clippy's `needless_return` lint, which would flag it, is turned off (see
[lints](#lints-clippy-and-rustdoc)).

## Part 2: Data

### Structs and enums with data

A `struct` is a record with named fields. An `enum` is one of several
*variants*, and unlike enums in Java or C, each variant can carry its own
data. `Request` in `crates/ripandload-core/src/start.rs`:

```rust
pub enum Request {
    /// Check tools.
    CheckTools,
    /// A Download.
    Download {
        /// The pasted URLs, downloaded in this order.
        urls: Vec<String>,
        /// The Quality for every Source in this Download; `None` keeps the
        /// Quality in Settings.
        quality: Option<u8>,
    },
    /// An Upload of the Staging folder to the active Server.
    Upload,
```

A `Request` is a Download *with* its URLs and Quality, or one of the others,
with nothing. Code that gets a `Request` must say what to do for each
variant (see [`match`](#match)).

A pattern can take a struct apart into its fields:
`for Entry { path, is_dir } in …` gives each field its own name.

Seen in: [Upload: clearing](pipelines/upload.md#9-clearing).

### `match`

`match` compares a value against patterns, in order, and runs the first arm
that fits. It must cover every case, or the code doesn't compile.
`Run::status` in `crates/ripandload-core/src/run.rs`:

```rust
match (&self.current, self.cancelled, self.problems) {
    (Some((spec, _)), _, _) => format!("{name}: running {spec}…"),
    (None, true, _) => format!("{name} cancelled"),
    (None, false, 0) if self.kind.says_finished() => format!("{name} finished"),
    (None, false, 0) => format!("{name}: all good"),
    (None, false, 1) => format!("{name}: 1 problem, see the log"),
    (None, false, n) => format!("{name}: {n} problems, see the log"),
}
```

It matches a tuple of three values at once. `_` matches anything. `n` matches
anything too, and names it. `if …` after a pattern is a *guard*: the arm only
fits when the guard is also true. `match` is an expression, so the whole
thing is the function's value.

Seen in: [Compare: the end](pipelines/compare.md#6-the-end).

### Slice patterns

A pattern can match the *shape* of a list: how many elements, and what's
in them. `decode_wsl_output` in `crates/ripandload-core/src/wsl_output.rs`
reads bytes two at a time:

```rust
.map(|pair| match pair {
    [low, high] => u16::from_le_bytes([*low, *high]),
    _ => char::REPLACEMENT_CHARACTER as u16,
})
```

`[low, high]` fits a pair of exactly two bytes and names them. The last
chunk of an odd-length input has one byte, so it falls to `_`.

Seen in: [URL recognition: reading the URL](pipelines/url-recognition.md#4-reading-the-url).

### `if let`, `let … else` and let chains

`if let` runs a block only when a value fits one pattern, and names its
parts. `let … else` is the reverse: it names the parts, and when the value
doesn't fit, runs the `else` block, which must leave the function.
`RipAndLoadApp::confirm_clear` in `crates/ripandload/src/app.rs`:

```rust
let Some(ClearPrompt {
    music,
    other,
    after_upload,
}) = self.confirming_clear
else {
    return;
};
```

If no dialog is open, the function ends. Otherwise `music`, `other` and
`after_upload` are named for the rest of it.

A *let chain* joins `let` patterns and conditions with `&&`. `upload_tab::show`
in `crates/ripandload/src/upload_tab.rs`:

```rust
if let Ok(contents) = contents
    && !contents.groups.is_empty()
{
```

The block runs only if reading the Staging folder worked *and* found some
music.

Seen in: [Check tools: reading rip's line](pipelines/checks.md#5-reading-rips-line).

### `Option` and the question mark

There is no `null`. A value that may be missing is an `Option`: `Some(x)`
or `None`. The compiler won't let code use `x` without handling `None`.

`?` after an `Option` means: if it's `None`, return `None` from this
function now. `command::parse_rip_version` in
`crates/ripandload-core/src/command.rs`:

```rust
pub fn parse_rip_version(line: &str) -> Option<&str> {
    let (_, version) = line.trim().split_once(", version ")?;
    Some(version.trim()).filter(|version| !version.is_empty())
}
```

`split_once` gives `None` when the line has no `, version `, and `?` then
ends the function with `None`. `filter` turns `Some` into `None` when the
test fails.

Seen in: [URL recognition: recognising the Source](pipelines/url-recognition.md#1-recognising-the-source).

### `Result` and error enums

There are no exceptions. A function that can fail returns a `Result`:
`Ok(value)` or `Err(error)`. `?` after a `Result` means: if it's an `Err`,
return it from this function now, converted to this function's error type
if needed. `map_err` changes the error on the way.

The errors are enums, one per area, in `crates/ripandload-core/src/error.rs`.
`thiserror` writes each variant's message from `#[error(...)]`, and
`#[source]` marks the error that caused it:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// The tool's executable isn't on `PATH`.
    #[error("`{tool}` wasn't found on PATH; {hint}")]
    NotFound {
        // …
        #[source]
        source: which::Error,
    },
```

`error_chain`, in the same file, follows the causes to build one line:

```rust
let mut text = readable(err);
let mut source = err.source();
while let Some(cause) = source {
    text.push_str(": ");
    text.push_str(&readable(cause));
    source = cause.source();
}
```

`while let` loops as long as the value fits the pattern.

Seen in: [URL recognition: asking the API](pipelines/url-recognition.md#5-asking-the-api),
[Download: starting the Run](pipelines/download.md#2-starting-the-run),
[architecture: error handling](architecture.md#error-handling).

### Derive, the `Debug` form, and serde

`#[derive(...)]` asks the compiler to write the code for some standard
traits (see [traits](#traits-and-default-methods)). `QualityDefaults` in
`crates/ripandload-core/src/settings.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QualityDefaults {
    /// The Qobuz Quality level, 1 to 4.
    pub qobuz: u8,
```

- `Debug` prints the value for developers. It's the form the documents show
  values in: `QualityDefaults { qobuz: 3, deezer: 2 }`, or for an enum
  variant, `Downloaded`.
- `Clone` gives a `.clone()` that copies the value.
- `PartialEq` gives `==`.
- `Serialize` and `Deserialize` come from serde. They let `toml` read the
  struct from a file and write it back, a key per field.
  `#[serde(default)]` fills a key missing from the file with the
  `Default` value.

Seen in: [Settings: loading](pipelines/settings.md#1-loading).

## Part 3: Ownership

### Owned and borrowed

Every value has one *owner*, and is freed when its owner goes away. Passing
a value hands ownership over (a *move*). Passing `&value` lends it instead,
read-only; `&mut value` lends it to be changed. The compiler checks that no
borrow outlives the value, and that a value isn't changed while it's lent.

`Upload::new` in `crates/ripandload-core/src/run/upload.rs` borrows the
`ToolEnv` and the `Server`, and takes the Staging folder's `PathBuf` as its
own, since the Upload keeps it:

```rust
pub(crate) fn new(
    tool_env: &ToolEnv,
    server: &Server,
    staging: PathBuf,
```

`Option::take` moves the value out of an `Option` and leaves `None`. That's
how `Upload::outcome` hands its snapshot over exactly once:

```rust
match self.at_start.take() {
```

### Lifetimes, as far as reading them

A *lifetime* is a name for how long a borrow lives, written `'a`. Most are
left out, and the compiler works them out. Two forms show up here.

`'static` means "lives as long as the program", as a string literal does.
Every kind of Run returns its name that way, from `trait Kind` in `run.rs`:

```rust
fn name(&self) -> &'static str;
```

A named lifetime ties borrows together. `Comparison::to_upload` in
`crates/ripandload-core/src/staging.rs` takes paths that must live at
least as long as `'a`:

```rust
pub fn to_upload<'a>(&self, files: impl IntoIterator<Item = &'a Path>) -> usize {
```

Reading code, it's enough to know that `'a` is about how long something
lives, never about what it is.

### `Drop`

`Drop` is code that runs when a value goes away, like a destructor in C++.
A `Process` cancels itself, so the OS process it started is never left
running. In `crates/ripandload-core/src/process.rs`:

```rust
impl Drop for Process {
    fn drop(&mut self) {
        self.cancel();
    }
}
```

Seen in: [Download: where the threads are](pipelines/download.md#where-the-threads-are).

## Part 4: Abstraction

### Traits and default methods

A *trait* is a set of methods a type can implement, like an interface in
Java. A method can have a default body, which an implementation keeps
unless it writes its own. Part of `trait Kind` in
`crates/ripandload-core/src/run.rs`:

```rust
pub(crate) trait Kind {
    /// Returns the run's name, as the status bar shows it.
    fn name(&self) -> &'static str;
    // …
    /// Returns whether the run ends at its first problem.
    fn stops_at_first_problem(&self) -> bool {
        return true;
    }
```

Every kind must write `name`. Only Check tools writes its own
`stops_at_first_problem`.

Standard traits work the same way. `Display` is how a value prints for
users; `Source` in `source.rs` implements it:

```rust
impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Source::Qobuz => "Qobuz",
            Source::Deezer => "Deezer",
        })
    }
}
```

Seen in: [Download: spawning](pipelines/download.md#5-spawning),
[architecture: the life of a Run](architecture.md#the-life-of-a-run).

### `Box<dyn Kind>` and `impl …` arguments

`dyn Kind` is "some type that implements `Kind`, decided at run time". Its
size isn't known, so it lives on the heap, in a `Box`. That's how one `Run`
holds any kind. `start::boxed` in `crates/ripandload-core/src/start.rs`
turns a concrete kind into one:

```rust
fn boxed<K: Kind + 'static>(
    (kind, specs): (K, Vec<CommandSpec>),
) -> (Box<dyn Kind>, Vec<CommandSpec>) {
    return (Box::new(kind), specs);
}
```

`K` is a type parameter, any type that implements `Kind`.

`impl Trait` as an argument type is shorter: "any type that implements
this", decided at compile time. `Upload::new` takes the function that reads
the Staging folder that way, so tests can pass their own:

```rust
read: impl FnOnce(&Path) -> Result<StagingContents, CantStart>,
```

`FnOnce` is the trait of anything callable once: a function or a closure.

### Closures and `move`

A *closure* is an anonymous function, written `|args| body`, that can use
the variables around it. By default it borrows them. `move` makes it take
them over instead, which a closure handed to another thread must do: the
thread may outlive the function that started it. `forward_chunks` in
`crates/ripandload-core/src/process.rs`:

```rust
thread::spawn(move || {
    let mut buffer = [0; 4096];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
```

The closure owns `reader` and the channel's `sender` from then on.

Seen in: [URL recognition: starting a lookup thread](pipelines/url-recognition.md#3-starting-a-lookup-thread).

### Iterators

An *iterator* produces values one at a time. Methods such as `map`,
`filter` and `flat_map` build a new iterator from an old one, and nothing
runs until something consumes it, with `collect`, `count` or a `for` loop.
`StagingContents::music_files` in `crates/ripandload-core/src/staging.rs`:

```rust
pub fn music_files(&self) -> impl Iterator<Item = &Path> {
    self.groups
        .iter()
        .flat_map(|group| &group.files)
        .map(PathBuf::as_path)
}
```

It walks every group, then every file in each, without building a list.
An `Option` is iterable too, with zero or one value, which is how an
optional command is put in front of a list with `chain`.

Seen in: [Compare: starting the Run](pipelines/compare.md#2-starting-the-run).

## Part 5: Concurrency

### Threads and `Arc<AtomicBool>`

`thread::spawn` starts an OS thread. Two threads can't share a plain value:
the compiler won't allow it. `Arc` is a shared pointer that both can own,
counting its owners. `AtomicBool` is a `bool` one thread can set while
another reads it, without a lock.

A `Process` and its exit thread share an `Arc<AtomicBool>`. `Process::cancel`
sets it:

```rust
self.cancelled.store(true, Ordering::Relaxed);
```

and `wait`, on the exit thread, checks it every 20 ms:

```rust
if cancelled.load(Ordering::Relaxed) {
    let _ = child.kill();
    let _ = child.wait();
    return Exit::Cancelled;
}
```

Shared data that isn't a single flag needs a `Mutex`, a lock. The app uses
one, around the Log file in `main.rs`, so threads writing logs take turns.

There's no `async`: see [architecture: concurrency](architecture.md#concurrency).

Seen in: [Download: the process's threads](pipelines/download.md#6-the-processs-threads).

### Channels and `try_recv`

A *channel* carries values from threads to one receiver, in order.
`mpsc::channel()` returns its two ends. The `Sender` can be cloned, one per
thread. The `Previews` in `crates/ripandload/src/download_tab.rs` make
theirs:

```rust
let (sender, receiver) = mpsc::channel();
```

A thread sends, and stops when nobody listens any more, as in
`forward_lines` in `process.rs`:

```rust
if sender.send(ProcessEvent::Line(line)).is_err() {
    break;
}
```

The receiver has `recv`, which waits, and `try_recv`, which doesn't: it
returns the next value, or `Empty` when there is none yet, or
`Disconnected` when every sender is gone. The GUI thread only ever uses
`try_recv` (or `try_iter`, which takes everything waiting), so it never
stops a frame. A `Process` hands out its receiving end:

```rust
pub fn events(&self) -> &Receiver<ProcessEvent> {
    &self.events
}
```

Seen in: [URL recognition: back to the UI thread](pipelines/url-recognition.md#7-back-to-the-ui-thread),
[Download: each frame](pipelines/download.md#7-each-frame),
[architecture: concurrency](architecture.md#concurrency).

## Part 6: Building and checking

### Cargo workspaces and dependency features

A Cargo *workspace* builds several crates together, sharing one lock file
and one `target/` folder. The root `Cargo.toml` holds what they share:

```toml
[workspace.package]
version = "0.1.0"
edition = "2024"
```

and each crate takes it:

```toml
edition.workspace = true
```

The *edition* is the version of the language's rules the code is written
for.

A dependency can have optional *features*, turned on where it's declared:

```toml
serde = { version = "1", features = ["derive"] }
```

serde's `derive` feature is what makes `#[derive(Serialize, Deserialize)]`
available.

Seen in: [crates: workspace mechanics](crates.md#workspace-mechanics).

### `cfg`

`cfg` asks about the build: which OS, whether it's a test build. As
`cfg!(…)`, it's a `bool` in ordinary code, and both branches are compiled.
`decode_tool_output` in `crates/ripandload-core/src/wsl_output.rs`:

```rust
if cfg!(windows) {
    decode_wsl_output(bytes)
} else {
    String::from_utf8_lossy(bytes).into_owned()
}
```

As an attribute, `#[cfg(…)]` leaves the item out of the build entirely when
it's false. A test in `staging.rs` that needs Unix symlinks:

```rust
#[cfg(unix)]
#[test]
fn symlinks_are_counted_not_followed() {
```

Seen in: [Download: inside WSL](pipelines/download.md#example-b-the-same-download-inside-wsl),
[Settings: saving](pipelines/settings.md#5-saving).

### Tests

A function marked `#[test]` is a test: it passes unless it panics. Unit
tests sit in a `mod tests` at the bottom of the module they test, compiled
only for `cargo test`. From `crates/ripandload-core/src/run/upload.rs`:

```rust
#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
```

`#[allow(…, reason = …)]` turns a lint off for that block, and says why.
A test that needs something special is skipped unless asked for, as in
`crates/ripandload-core/tests/ssh_target.rs`:

```rust
#[ignore = "needs dev/ssh-target.sh"]
```

→ [architecture: testing](architecture.md#testing)

### Lints, clippy and rustdoc

A *lint* is a check for code that compiles but probably shouldn't look that
way. clippy is Rust's large collection of extra lints. The root
`Cargo.toml` sets them for both crates:

```toml
[workspace.lints.clippy]
missing_docs_in_private_items = "warn"
# …
# Explicit `return`s are kept for readability.
needless_return = "allow"
```

rustdoc builds the API docs from the doc comments, and checks their links.
`dev/check.sh` runs both, with every warning denied.

## Reading one signature

Here is `Upload::new` from `crates/ripandload-core/src/run/upload.rs`,
whole:

```rust
pub(crate) fn new(
    tool_env: &ToolEnv,
    server: &Server,
    staging: PathBuf,
    read: impl FnOnce(&Path) -> Result<StagingContents, CantStart>,
) -> Result<(Self, Vec<CommandSpec>), CantStart> {
```

Piece by piece:

- `pub(crate)`: callable anywhere in the core crate, not outside it.
  → [visibility](#crates-modules-use-and-visibility)
- `fn new(`: a function named `new`. In Rust that's only a habit for
  constructors, not a keyword.
- `tool_env: &ToolEnv` and `server: &Server`: borrowed, read-only. The
  caller keeps them. → [owned and borrowed](#owned-and-borrowed)
- `staging: PathBuf`: owned. The caller hands the path over.
- `read: impl FnOnce(&Path) -> …`: any function or closure that takes a
  borrowed path once and returns a `Result`.
  → [`impl …` arguments](#boxdyn-kind-and-impl--arguments)
- `-> Result<…, CantStart>`: it succeeds, or says why the Upload can't
  start. → [`Result`](#result-and-error-enums)
- `(Self, Vec<CommandSpec>)`: on success, a tuple of two values. `Self` is
  the type the function belongs to, `Upload`. `Vec<CommandSpec>` is a list
  of commands to run. → [structs and enums](#structs-and-enums-with-data)
