# The app owns a separate Streamrip config

The app never edits the user's personal streamrip `config.toml`. It keeps its own Streamrip config file, seeded once from the user's file if one exists, and passes it to every run with `rip --config-path`. This means the app's Settings can never break a hand-tuned personal setup, the app always knows exactly which file it controls, and on Windows the file can live inside WSL next to the venv. The file is edited with `toml_edit` so comments and keys the app doesn't manage are preserved.
