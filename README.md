# ripandload

A desktop app for a group of friends that share one [Navidrome](https://www.navidrome.org/) server. Paste a Qobuz or Deezer link and press **Download**: [streamrip](https://github.com/nathom/streamrip) saves the music into a folder on your computer, the Staging folder. Press **Upload** and rsync copies that folder to the Server's music folder. Runs on Linux and Windows 11.

Upload only adds files. Nothing on the Server is ever deleted.

## What you need

- A Qobuz or Deezer account, and its Credentials (see [Credentials](#credentials)).
- An SSH login to the Server: a host, a user, and an SSH key that lets you in without a password. Ask whoever runs the Server.
- streamrip 2.2.0, rsync and ssh. On Windows these run inside WSL (see below).

## Installing

### Windows 11

1. Set up WSL once, following [docs/wsl-setup.md](docs/wsl-setup.md). It takes about ten minutes.
2. Put `ripandload.exe` anywhere you like (your Desktop is fine) and double-click it. There is nothing else to install.

Windows may warn that the app is from an unknown publisher. Click **More info → Run anyway**.

### Linux

1. Install rsync and ssh with your package manager (on Arch: `sudo pacman -S rsync openssh`; on Debian/Ubuntu: `sudo apt install rsync openssh-client`).
2. Install streamrip 2.2.0, for example with `pipx install streamrip==2.2.0`.
3. Run the `ripandload` binary, or build it yourself (see [Building](#building)).

## First start: Settings

Open the **Settings** tab, fill it in, and press **Save**. A "• unsaved changes" marker shows until you do, and the app asks before quitting with unsaved changes.

- **Credentials**: Qobuz user ID and auth token, or Deezer ARL. You only need the Source(s) you use.
- **Quality**: the default Quality per Source. You can override it for a single Download.
- **Naming templates**: how downloaded folders and tracks are named. Click a placeholder to add it. The defaults are streamrip's own.
- **Staging folder**: where Downloads land. The default is `Music/ripandload` in your home folder.
- **WSL distro** (Windows): the distro you set up for ripandload. Leave it empty to use WSL's default distro.
- **streamrip inside WSL** (Windows) or **streamrip executable** (Linux): leave it empty unless streamrip lives somewhere unusual.
- **Server**: host, user, port, the Server's music folder (e.g. `/srv/music`), and your SSH key file.

Then press **Check tools** and **Test connection** (see [Troubleshooting](#troubleshooting)). If both are happy, you're ready.

### Credentials

ripandload passes your Credentials to streamrip. streamrip's wiki explains how to find them: the Qobuz user ID and auth token from a logged-in Qobuz web player, or the Deezer ARL cookie. Treat them like a password. They are stored only in your Settings file.

## Using it

**Download**: paste one or more album, track or playlist URLs, one per line, and press **Download**. Each line shows which Source it was recognised as. Pick a Quality if you want something other than your default. Output appears in the log at the bottom, and **Cancel** stops it.

**Upload**: shows the Server and how many files are waiting in the Staging folder. Press **Upload** to copy them all. When it finishes, the app offers to clear the Staging folder. **Clear…** does the same at any time. Clearing only deletes your local copy.

One Download or Upload runs at a time.

## Where things live

| | Linux | Windows |
|---|---|---|
| Settings | `~/.config/ripandload/settings.toml` | `%APPDATA%\ripandload\config\settings.toml` |
| Streamrip config | `~/.config/ripandload/streamrip.toml` | `%APPDATA%\ripandload\config\streamrip.toml` |
| Log file | `~/.local/share/ripandload/ripandload.log` | `%APPDATA%\ripandload\data\ripandload.log` |
| Staging folder | `~/Music/ripandload` | `%USERPROFILE%\Music\ripandload` |

To open a `%APPDATA%` path on Windows, paste it into the File Explorer address bar.

The Streamrip config belongs to the app. ripandload rewrites your Credentials, Quality, Naming templates and folders in it before every Download. Other streamrip options you change there are kept. On Linux, the first Download copies it from your own streamrip config (`~/.config/streamrip/config.toml`) if you have one. Your own config is never changed.

## Troubleshooting

Start with the two buttons on the Settings tab. They use the Settings as they are on screen, saved or not.

**Check tools** runs `rip --version` and `rsync --version` (on Windows, inside WSL).

- `✓ streamrip 2.2.0` and an rsync version: all good.
- "streamrip … found, but ripandload is made for 2.2.0": it may work, but install 2.2.0 if anything goes wrong.
- "couldn't run anything inside WSL": WSL isn't installed, or the WSL distro in Settings doesn't exist. Run `wsl -l` in PowerShell to see your distros' names.
- "streamrip isn't where Settings say inside WSL": redo step 3 of [docs/wsl-setup.md](docs/wsl-setup.md), or set **streamrip inside WSL**. Check tools lists any pyenv installs it finds.
- "`rip` wasn't found on PATH" (Linux): install streamrip, or set **streamrip executable**.

**Test connection** logs in to the Server and checks that the music folder is writable.

- "couldn't reach or log in to the Server": check the host, port, user and SSH key. The key must work without a password prompt. A key with a passphrase only works through ssh-agent.
- "the music folder doesn't exist or isn't writable": check the folder's path with whoever runs the Server.

**A Download fails.** Read the log. streamrip prints why: usually expired Credentials, or a link to something your account can't stream. If the message mentions the Streamrip config, delete that file (see [Where things live](#where-things-live)). The next Download makes a new one.

**The first command after Windows starts is slow.** WSL is starting up. That takes a few seconds.

**Still stuck?** Send the log file (see [Where things live](#where-things-live)) to whoever gave you the app. It holds this run and the previous one, so send it before restarting the app too many times. It contains the commands that ran and their output, but not your Credentials.

## Building

### Linux

```sh
cargo run -p ripandload            # debug build
cargo build -p ripandload --release
```

The release binary is `target/release/ripandload`.

### Windows (cross-compiled from Arch Linux)

One-time setup:

```sh
rustup target add x86_64-pc-windows-gnu
sudo pacman -S --needed mingw-w64-gcc
```

Build:

```sh
cargo build -p ripandload --release --target x86_64-pc-windows-gnu
```

The result is a single file, `target/x86_64-pc-windows-gnu/release/ripandload.exe`, with no extra DLLs to ship.

Release builds on Windows open no console window. To see logs live, build with `--profile release-console` instead. The window renders with OpenGL. If it misbehaves, set `RIPANDLOAD_RENDERER=wgpu` to try wgpu.

### Checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Licence

MIT
