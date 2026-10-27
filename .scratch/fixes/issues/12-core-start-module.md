# 12 – Core `start`: one way to start a Run

Status: ready-for-agent
Blocked by: 11

Part of `../spec.md`. Designed in `../../architecture/issues/02-run-start-up.md` (see its Answer). Names follow fix 11 (`Run`, `Process`).

## Scope

1. New core module `start.rs`:
   - `pub enum Request { CheckTools, Download { urls: Vec<String>, quality: Option<u8> }, Upload, Compare, TestConnection }`: only what Settings don't hold.
   - `pub struct Places { streamrip_config: Option<PathBuf>, personal: Option<PathBuf> }`, with a constructor from `streamrip_config::default_path()` / `personal_path()`.
   - `pub fn start(settings: &Settings, request: Request, places: &Places, log: &mut Vec<String>) -> Result<Run, CantStart>`. It clears the log, then builds the kind's commands and starts the Run. On a `CantStart` it leaves the log as `✗ <reason>` only.
   - `pub fn server(settings: &Settings) -> Result<&Server, CantStart>`: no active Server, or an empty host or music folder (moves `app.rs`'s `server_missing` rule to core). Used by `start` for Upload, Compare with Server and Test connection.
2. `CantStart` in `error.rs` (thiserror, `#[source]` where wrapping): `NoServer`, `ServerIncomplete { host: bool, music_folder: bool }` (messages as `server_missing`'s today: "Set the Server's host first." …), `NoConfigFolder`, `StreamripConfig(StreamripConfigError)`, `Tool(ToolError)`. `start` doesn't call `Settings::validate`.
3. Every failure before the first process spawns is a `CantStart`: no Server, incomplete Server, no config folder (Download only), the Streamrip config render, `tool_path`, `ssh_ready`. A process that fails to spawn still gives a Run finished at once with `✗` in the log.
4. Download start-up moves from `app.rs` into `start`: render the Streamrip config at `places.streamrip_config` (seeding from `places.personal`), log "Created the Streamrip config at … from …" when it was new, translate its path with `tool_path`, build `download_spec`.
5. `start` builds every kind's spec list (Check tools' WSL check plus version specs; the Download spec; copy-key plus upload/compare; copy-key plus the ssh checks). `Run` gets one `pub fn new(kind, tool_env, specs: Vec<CommandSpec>, log)` (made `pub(crate)` in fix 13); `Kind` becomes visible to `start`. The pyenv probe added mid-run stays in `Run`. The five old public constructors stay, delegating, until fix 13 removes them.
6. Tests: `tests/run.rs` and `tests/ssh_target.rs` go through `start` with Settings and a `Places` in a temp dir (personal = the existing fixture, so seeding doesn't run `rip`). Add `common::settings_with_script_rip(dir, script)`. Drop the hand-made `rip_spec`. One test per `CantStart` variant, with no process.

## Acceptance

- [ ] Every `CantStart` variant has a test; Download start-up is tested without egui or the real config folder.
- [ ] Runs that exist have always spawned a process (or failed to spawn one).
- [ ] `dev/check.sh` passes; the `#[ignore]`d SSH-target tests pass against `dev/ssh-target.sh` if available, else say so under Comments.

## Comments
