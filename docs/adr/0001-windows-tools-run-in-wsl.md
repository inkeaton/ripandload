# On Windows, streamrip and rsync run inside WSL

On Windows the app does not use native Windows builds of Python, streamrip or rsync. It runs them inside a WSL distro, with streamrip in a dedicated venv, by calling `wsl.exe -d <distro> -- ...`. rsync has no first-class native Windows build, and a single Linux toolchain behaves the same as on our Arch development machines, so there is one set of command lines to get right. The costs are that WSL is a prerequisite, Windows paths must be translated to `/mnt/<drive>/...`, and SSH keys must be copied into WSL to get `0600` permissions.
