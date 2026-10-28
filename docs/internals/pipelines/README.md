# Pipelines

Each pipeline follows one thing the user does through the code, from the
click to what they see. Read them in this order: the later ones link back to
the earlier ones rather than explain the same code twice.

1. [URL recognition and Preview](url-recognition.md): a pasted URL gets its
   Source, then a Preview from the Source's API.
2. [Download](download.md): `rip` runs on the pasted URLs. This is where
   processes, threads and channels are explained.
3. [Compare with Server](compare.md): an rsync dry run finds what the
   Server lacks. This is where the rsync command line is explained.
4. [Upload](upload.md): rsync sends the Staging folder, and the app offers
   to clear it.
5. [Settings round-trip](settings.md): the Settings file and the app, then
   the Settings to the Streamrip config.
6. [Check tools and Test connection](checks.md): whether the tools are
   there, and whether the Server can be reached.

The terms they use are defined in the [glossary](../../../GLOSSARY.md).

## How to read a pipeline

Every pipeline has the same parts, in the same order.

- **A diagram.** A Mermaid sequence diagram of the whole path, at the top.
- **The examples.** Two to four concrete cases, one line each. At least one
  is a failure.
- **Example A, step by step.** The main example, the richest path that
  works, traced through every step. Each step names the function and its
  file, says who calls it and on which thread, shows a short excerpt of the
  one decision it makes, and shows the values going in and out, in Rust's
  `Debug` form. A step may stop to say *why* the code is the way it is.
- **The other examples.** Each says only where it leaves example A: "Steps 1
  to 3 go as for A. In step 4 it differs." They show code only where A
  didn't reach it.
- **Rust notes.** A `> **Rust:**` note explains, in a sentence or two, a
  Rust feature the first time it matters in that document, and links to the
  [Rust primer](../rust-primer.md).
- **Where the threads are.** Which steps run on which thread, and how the
  threads talk.
- **Tested by.** The last line names the tests that check this path.

Code is named by item and file, `preview::parse` in
`crates/ripandload-core/src/preview.rs`, never by line number. The excerpts
are trimmed, with `// …` where code was left out.

The tools the app runs, streamrip's `rip`, rsync, ssh and `wsl.exe`, are
black boxes here: the pipelines show the command lines the app builds, the
output it reads, and the exit codes it acts on, not how the tools work.
