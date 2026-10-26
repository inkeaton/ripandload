# Setting up WSL for ripandload (Windows 11)

On Windows, ripandload runs streamrip, rsync and ssh inside WSL (see `adr/0001-windows-tools-run-in-wsl.md`). Do this once per computer.

## 1. Install WSL and a distro

In PowerShell:

```powershell
wsl --install -d Debian
```

Restart if asked. Then open **Debian** from the Start menu once and create your Linux user. If you already use another distro (Ubuntu, …), you can use that instead. Put its name under Settings → WSL distro, or leave the field empty to use WSL's default distro.

## 2. Install rsync, ssh and Python

(`procps` provides `pkill`, which Cancel uses.)

Inside the distro:

```sh
sudo apt update
sudo apt install rsync openssh-client procps python3-venv
```

## 3. Install streamrip 2.2.0

### Option A: the ripandload venv (default)

```sh
python3 -m venv ~/.local/share/ripandload/venv
~/.local/share/ripandload/venv/bin/pip install streamrip==2.2.0
```

The app looks for `rip` there by default, so leave Settings → streamrip inside WSL empty.

### Option B: pyenv

If you manage Python with [pyenv](https://github.com/pyenv/pyenv), install streamrip into a pyenv version or virtualenv:

```sh
pyenv install 3.12
pyenv virtualenv 3.12 streamrip      # needs the pyenv-virtualenv plugin
~/.pyenv/versions/streamrip/bin/pip install streamrip==2.2.0
```

Then set Settings → streamrip inside WSL to that environment's `rip`, for example `~/.pyenv/versions/streamrip/bin/rip`.

The app starts tools without your `.bashrc`, so pyenv's `PATH` setup doesn't apply and a plain `rip` isn't found. Point at the environment's `bin/rip` directly. `~/.pyenv/shims/rip` also works, but only if that environment is pyenv's global version (`pyenv global`). If Check tools can't find `rip`, it lists any it finds under `~/.pyenv`.

## 4. Check

In the app, press Settings → **Check tools**. It should show `✓ streamrip 2.2.0` and rsync's version.

## Notes

- The Staging folder must be on a drive (e.g. `C:\Users\you\Music\ripandload`, the default). rip and rsync see it as `/mnt/c/Users/you/Music/ripandload`.
- The SSH key can be anywhere on Windows. Before each Upload or Test connection, the app copies it to `~/.ssh/ripandload_key` inside WSL with mode 600, because ssh refuses keys on `/mnt/c`.
- The first command after Windows starts can take a few seconds while WSL boots.
