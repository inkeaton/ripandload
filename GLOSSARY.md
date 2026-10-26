# ripandload

A desktop frontend that downloads music with streamrip and uploads it to a friend group's Navidrome server.

## Language

### Getting music

**Source**:
A streaming service streamrip downloads from (Qobuz, Deezer, Tidal, SoundCloud).
_Avoid_: Provider, platform, service

**Credentials**:
The per-Source secrets that authorise downloads (Qobuz user ID and auth token, Deezer ARL).
_Avoid_: Access token, login, keys

**Quality**:
The maximum audio quality level requested for a Download.
_Avoid_: Bitrate, format

**Naming template**:
The pattern that names downloaded folders and track files from their metadata.
_Avoid_: File/folder name, path format

**Download**:
One run of streamrip over one or more pasted URLs, writing into the Staging folder.
_Avoid_: Rip, job, fetch

**Staging folder**:
The local folder where Downloads land and from which Uploads are made.
_Avoid_: Download folder, temp folder, inbox

**Preview**:
The kind, name and artist looked up for a pasted URL before a Download. It's only information and never blocks one.
_Avoid_: Metadata, lookup, info

### Sharing music

**Server**:
The remote machine running Navidrome, with the login, music folder and SSH key used to reach it.
_Avoid_: Remote, host, laptop, NAS

**Upload**:
One push of the whole Staging folder to a Server's music folder.
_Avoid_: Sync, transfer, deploy

**Music file**:
An audio file in the Staging folder, told apart by its extension. Covers, lyrics and cue sheets aren't music files, though Uploads still send them.
_Avoid_: Song, audio

**Album folder**:
The folder that directly holds a group of music files: an album or a playlist, with any disc folders inside it. Music files directly in the Staging folder are the single tracks.
_Avoid_: Release, collection

**Comparison**:
What a Compare with Server found: which music files the Server already has and which an Upload would send. A successful Upload leaves one where the Server has everything.
_Avoid_: Sync status, diff

### The app

**Settings**:
Everything the user configures in the app: Credentials, Quality, Naming templates, Staging folder, Server.
_Avoid_: Config, preferences

**Streamrip config**:
The app-owned streamrip configuration file generated from Settings; distinct from the user's personal streamrip config.
_Avoid_: rip config, config.toml
