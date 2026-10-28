# 02 – Outline of architecture.md

Type: grilling
Status: resolved
Blocked by: none

Part of `../map.md`.

## Question

What does each section of `architecture.md` (listed in the map's Notes) say, how deep does it go, and which diagrams does it have (a crate and module diagram? a Run life-cycle state diagram? a thread diagram)? Also, which sections point to which pipeline, and where testing (`tests/common` and its fake tools) is covered.

## Answer

**Style.** Each section is a short overview: about 3–8 short paragraphs in the plain-sentence style of the pipelines (see [01](01-pipeline-format-url-recognition.md)). It has at most one or two trimmed excerpts, used only where a shape can't be shown any other way (the fields of `Run`, the order of work in a frame). It adds `> **Rust:**` callouts where an idiom first appears and ends with a "Goes deeper in: <pipeline>" link.

**Sections, in order** (with their pipeline links):

1. **The app in one page.** What the app does in glossary terms: Source → Download → Staging folder → Upload → Server. Diagram: a context `flowchart` showing the user, the GUI crate, the core crate and the black boxes (rip, rsync, ssh, `wsl.exe`, the Qobuz/Deezer APIs used for Preview, the Server). Each edge is labelled with what crosses it: command line, output lines, exit code, HTTP.
2. **The two-crate split.** `ripandload` draws the window; `ripandload-core` knows nothing about the GUI, so everything in it can be tested without a window. No pipeline link.
3. **Module map.** Diagram: two crate boxes with the core modules grouped as in `lib.rs` (Settings / Running tools / Reading output / Music / Problems) and the GUI modules (`app`, `download_tab`, `upload_tab`, `settings_tab`, `path_picker`, `wording`). Arrows show only the main calls, not every `use`. No pipeline link.
4. **The immediate-mode GUI loop.** Each frame, `RipAndLoadApp` polls the Run and then draws. Tabs return an `Action`, and the app saves Settings or calls `start::start`. Links: URL recognition and Preview (the text box settling, lookup rounds) and Settings round-trip (Actions, saving).
5. **Concurrency.** Threads, channels, `try_recv` polling, repaint. Diagram: the GUI thread, each `Process`'s stream-reader threads and exit thread, the Preview lookup thread, and the channels between them. Link: Download's "Where the threads are".
6. **The life of a Run.** `start::start` → `Run::new` → `poll` → `is_finished` → `take_outcome`. Covers `Kind` and `Note`, and stopping at the first problem (Check tools carries on). Diagram: a `stateDiagram` of the *derived* states. The text says plainly that `Run` has no state enum and that the states come from `current`, `queue`, `cancelled` and `outcome`. The states: `CantStart` (no Run exists) | running a command → next command → finished (succeeded / failed / cancelled) → Outcome taken. Links: Download (main trace), Check tools (carrying on after a problem).
7. **Error handling**, organised around where a problem shows up:
   - the **Status bar**: `CantStart` and the `error` enums, joined by `error_chain`;
   - the **Log**: the `✗`/`⚠`/`→` lines a Run writes from its kind's `Note`s;
   - the **Log file**: through `tracing`, with Credentials kept out by `error_chain`.

   Links: Check tools / Test connection (hints, `✗` lines), Settings round-trip (load and save errors).
8. **Native and WSL** (ADR 0001). `ToolEnv`, `wsl.exe -d <distro> --exec`, path translation in `wsl_path`, decoding output in `wsl_output`, the SSH key copy. Links: Download (command lines, path translation), Check tools (the WSL probe).
9. **Files on disk.** The one table allowed outside the no-tables rule: one row per file (Settings file, Streamrip config, Personal streamrip config (read-only), Log file and the previous one, Staging folder, the SSH key copied into WSL). The columns are owning module, Linux path, Windows path, and an ADR link where one applies (0002 for the Streamrip config). Links: Settings round-trip (Settings file, Streamrip config, seeding), Download (Staging folder).
10. **Testing.** The only full treatment of testing in the docs:
    - why the core crate is GUI-free;
    - unit tests inside modules versus integration tests in `crates/ripandload-core/tests/`;
    - `tests/common` and the shell-script stand-ins for `rip` (`fake-rip.sh`, `script-rip.sh`), and why they make those tests Unix-only;
    - the recorded fixtures (`rich13-*.out`, `preview/`);
    - `dev/check.sh`.

    No pipeline link.

**Amendment to the pipeline format (01):** each pipeline ends with a one-line "Tested by: <test file / test name>" after "Where the threads are". Testing is otherwise covered only in `architecture.md`.

**Amendment (from [05](05-compare-pipeline-outline.md)):** the Testing section also covers `tests/ssh_target.rs` and `dev/ssh-target.sh`:

- what it is: a Docker container with OpenSSH and rsync on `localhost:2222`, its music folder mounted from `target/ssh-target/music`, used as a real Server for Upload, Compare with Server and Test connection;
- why its tests are `#[ignore]`d by default and run with `--test-threads=1`: they need the container and share its music folder;
- why they're Unix-only: the tools run natively.
