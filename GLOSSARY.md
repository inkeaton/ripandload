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
The remote machine running Navidrome, with the login, music folder and SSH key used to reach it. Settings can hold several; the active Server is the one Runs use.
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

**Compare with Server**:
A Run that works out a Comparison against the active Server without sending anything.
_Avoid_: Check, dry run

**Test connection**:
A Run that checks the active Server can be reached over SSH with its key.
_Avoid_: Ping, connection check

**Comparison**:
What a Compare with Server found: which music files the Server already has and which an Upload would send. A successful Upload leaves one where the Server has everything.
_Avoid_: Sync status, diff

### The app

**Settings**:
Everything the user configures in the app: Credentials, Quality, Naming templates, Staging folder, Servers. Kept in the Settings file.
_Avoid_: Config, preferences

**Streamrip config**:
The app-owned streamrip configuration file generated from Settings; distinct from the user's personal streamrip config.
_Avoid_: rip config, config.toml

**Personal streamrip config**:
The user's own streamrip configuration file, which the app never edits. A new Streamrip config is seeded from it once.
_Avoid_: User config, rip config

**Run**:
One Check tools, Download, Upload, Compare with Server or Test connection. Only one Run goes at a time.
_Avoid_: Job, task, operation

**Check tools**:
A Run that checks rip and rsync start and that streamrip is the version the app is made for; inside WSL, first that WSL runs anything at all.
_Avoid_: Diagnostics, self-test

**Cancel**:
Stopping the current Run: its running command is stopped and the rest aren't started. A cancelled Run hasn't succeeded.
_Avoid_: Abort, stop, kill

**Log**:
The lines shown under the tabs: the current or last Run's commands and output, and the app's own messages. Cleared when a Run starts.
_Avoid_: Output, console

**Log file**:
The file on disk where the app records what happened across sessions, for diagnosing problems; not the same lines as the Log.
_Avoid_: Trace, debug log

**Live display**:
rip's progress bars during a Download, redrawn in place above the Log rather than appended to it.
_Avoid_: Progress output, live rows

**Status bar**:
The line at the bottom of the window saying what the app is doing, or why something didn't start.
_Avoid_: Status line
