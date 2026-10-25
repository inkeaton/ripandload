# ripandload

A desktop GUI that downloads music with [streamrip](https://github.com/nathom/streamrip) and uploads it to a Navidrome server with rsync. Runs on Linux and Windows 11.

## Building

### Linux

```sh
cargo run -p ripandload            # debug build
cargo build -p ripandload --release
```

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
