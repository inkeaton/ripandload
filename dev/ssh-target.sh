#!/bin/sh
# A throwaway SSH target (no Navidrome) for trying Upload and Test connection (ticket 07).
#
# Usage: dev/ssh-target.sh [start|stop]
#
# start: makes a key in target/ssh-target/, then runs linuxserver/openssh-server
#        with rsync, listening on localhost:2222. Upload lands in
#        target/ssh-target/music/ on this machine.
# stop:  removes the container (the key and the uploaded files stay).
#
# Settings to use:  Host localhost, User ripandload, Port 2222,
#                   Music folder /music, SSH key target/ssh-target/key
set -eu

name=ripandload-ssh-target
dir="$(cd "$(dirname "$0")/.." && pwd)/target/ssh-target"

case "${1:-start}" in
start)
    mkdir -p "$dir/music"
    [ -f "$dir/key" ] || ssh-keygen -q -t ed25519 -N '' -C "$name" -f "$dir/key"
    docker rm -f "$name" >/dev/null 2>&1 || true
    docker run -d --name "$name" \
        -p 127.0.0.1:2222:2222 \
        -e PUID="$(id -u)" -e PGID="$(id -g)" \
        -e USER_NAME=ripandload \
        -e PUBLIC_KEY="$(cat "$dir/key.pub")" \
        -e DOCKER_MODS=linuxserver/mods:openssh-server-rsync \
        -v "$dir/music:/music" \
        lscr.io/linuxserver/openssh-server:latest >/dev/null
    echo "Starting $name on localhost:2222 (installing rsync takes a few seconds)."
    echo "SSH key: $dir/key"
    echo "Uploads: $dir/music"
    ;;
stop)
    docker rm -f "$name" >/dev/null
    ;;
*)
    echo "usage: $0 [start|stop]" >&2
    exit 2
    ;;
esac
