# Smashcraft client

The desktop app players open: a window with routed pages (Controller,
History, Stats, Replays and Online; more slot into `ROUTES` in
smashcraft:client/ui/src/main.ts) and a tray light. Rust backend in
smashcraft:client/src-tauri (Tauri v2), pages in TypeScript built with Bun in
smashcraft:client/ui, embedded into the binary at build time.

## Controller support

The client never reads the pad itself. It connects to the Warcraft III
Controller service (smashcraft:companion) on 127.0.0.1:47631 and shows its
status, live input and bindings; the messages and plain-language status rows
come from smashcraft:companion/model. When nothing answers and the player has
turned controller support on (remembered in the client's settings), it starts
the bundled service with `--service`: `WC3_CONTROLLER_SERVICE`, else
`wc3-journal` beside the client, else on the `PATH`. A started service gets 10 s
to answer, three starts in a row, then the page says it couldn't start. The
service is single-instance, so a client never starts a second copy beside a
running one. `WC3_CONTROLLER_PORT` points the client at a test service.

Closing the window hides it to the tray; the tray's light is the overall
status and its menu opens the window or quits. Opening Smashcraft again while it
runs shows the running window (the first client holds 127.0.0.1:47632). "Start with my computer"
starts the client hidden in the tray at login (Linux: an XDG autostart entry
naming the installed launcher; Windows/macOS: the autostart plugin). Play runs
`bun wisp play` in `SMASHCRAFT_TS` or ~/code/smashcraft/main/ts and shows its
steps.

## Match history and stats

The map writes a record of every finished match into the player's
CustomMapData (smashcraft:docs/design/client.md, "Match records"). The History
page reads each configured folder (until the player sets them, every
Warcraft III CustomMapData found: under Documents, and on Linux in each
Steam/Proton prefix, ~/.wine and `WINEPREFIX`; remembered in the client's settings),
ingests new records into the client's own store, `history.json` in its data
folder, and lists the matches newest first. A record already stored (same
build, serial and writer) is skipped and one cut short is refused. The Stats
page counts versus matches from the writer's side: win rate per fighter and
per matchup, KOs and falls per match and damage dealt per match. Parsing,
ingest and stats are in smashcraft:client/ui/src/records.ts and
smashcraft:client/ui/src/stats.ts; the Rust side
(smashcraft:client/src-tauri/src/records.rs) only reads folders and keeps the
store. Fixture folders: smashcraft:client/ui/test/fixtures/records/.

## Online

The Online page hosts a private Battle.net game and shows its join code, or
joins one by code (smashcraft:docs/design/client.md, "Direct play"). It runs
`bun wisp online host` or `join CODE` from the same checkout as Play and
shows their output lines; their error output goes to `online.log` in the
client's log folder. Start now sends a waiting host `start` on its input, and
Cancel stops the run (a hosted game stays open until the player leaves it).
The first visit asks once to add Smashcraft's page to Warcraft III's menus
(`online setup`); the answer is remembered in the client's settings. The
runner is smashcraft:client/src-tauri/src/online.rs and the page
smashcraft:client/ui/src/pages/online.ts.

## Replays

Every client records each match as a replay in its CustomMapData folder: a
manifest, `smashcraft-replay-N.txt`, with its parts beside it
(smashcraft:docs/design/client.md, "Full-match replays"). The Replays page
lists the replays in the History page's folders, each beside the record of the
same match, and the replays the client keeps in `replays/` in its data
folder. "Save a copy to share" joins one into a single file there; "Open a
replay file…" plays a file copied from another computer and keeps it.

The viewer draws the replayed match (each fighter's hurt volumes, active
strikes and projectiles, damage and stocks) with play and pause (Space), a
frame back or forward (← →) and a seek bar. It runs the simulation of the
version that recorded the replay. The page build makes `sim.js` from
smashcraft:ts/src/game/replay/viewer.ts, stamped with its source version
(smashcraft:ts/scripts/sourceVersion.ts), and the client keeps a copy in
`sims/` for every version it runs. A replay from a version it doesn't hold
names that version. Joining and opening are in smashcraft:client/ui/src/replays.ts
and smashcraft:client/ui/src/playback.ts. The Rust side
(smashcraft:client/src-tauri/src/replays.rs) only reads and keeps files.

Warcraft III keeps only its last game's replay, `LastReplay.w3g` in each
account's `BattleNet/<id>/Replays` folder beside CustomMapData. While the
client runs it checks those files every 5 s; when one changes, and a match
record was written during that game (its length is in the file's header),
it keeps a copy in `warcraft-replays/` in its data folder, named
`warcraft-<UTC date>-<time>-<hash>.w3g` and listed in `games.json` with the
records it holds. The same content is kept once. Each match's row on the
Replays page then offers "Watch in Warcraft": it copies the game's replay
back into the Warcraft `Replays` folder it came from as
`Smashcraft <date> <time>.w3g` and tells the player to open it from
Warcraft III's Replays menu. A Warcraft replay covers the whole game, every
rematch included (smashcraft:client/src-tauri/src/warcraft_replays.rs).

## Build, test, install

```sh
(cd ts && bun install)   # the Replays page's simulation imports Wisp
cd client/ui && bun install && bun test && bun run check && bun run build
cd ../src-tauri && nix-shell ../shell.nix --run 'cargo test --jobs 2'
# Install for this user (NixOS): build, keep its libraries alive, write the
# launcher, desktop entry and refresh an enabled autostart entry.
cd .. && nix-shell shell.nix --run 'bun scripts/install.ts'
```

On NixOS put rustup's 1.96.1 toolchain on `PATH` first
(`$HOME/.rustup/toolchains/1.96.1-x86_64-unknown-linux-gnu/bin`). The install
lives in ~/.local/share/smashcraft-build-inputs/smashcraft-client (one folder
per commit, `current`, the `smashcraft` launcher and a `runtime` out-link), never
in the system configuration. CI (smashcraft:.github/workflows/client.yml)
builds and tests on Linux, Windows and macOS and uploads unsigned Windows and
macOS executables.
