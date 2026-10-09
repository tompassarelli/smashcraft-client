# Smashcraft client

The desktop app players install beside Warcraft III to play
[Smashcraft](https://github.com/tompassarelli/smashcraft): a window with
routed pages (Controller, History, Stats, Replays and Online; more slot into
`ROUTES` in ui/src/main.ts) and a tray light. Rust backend in src-tauri
(Tauri v2), pages in TypeScript built with Bun in ui, embedded into the binary
at build time. MIT licensed.

## Install

Download the release for your system from
[Releases](https://github.com/tompassarelli/smashcraft-client/releases) and
unpack it anywhere. It holds `smashcraft` (`smashcraft.exe`), the
[wc3-controller](https://github.com/tompassarelli/wc3-controller) service
beside it, and `VERSION` naming the commits it was built from. Run
`smashcraft`. On Linux the app needs WebKitGTK 4.1 and libayatana-appindicator
(`libwebkit2gtk-4.1-0 libayatana-appindicator3-1` on Debian and Ubuntu).

Put a Smashcraft map, `Smashcraft 0.0.N.w3x`, in Warcraft III's
`Maps/00-Smashcraft` folder. Play starts it. `smashcraft --check` prints,
without opening a window, what the controller service reports (its pad), the
bundled service it would start, and the map and command Play would run.

## What it reads from Smashcraft

Only Smashcraft's [client interface](https://github.com/tompassarelli/smashcraft/blob/main/docs/client-interface.md):

- at build time, client kit 1, written by the Smashcraft commit in
  `smashcraft.ref` (`bun scripts/kit.ts` fetches that commit's ts/ into
  .smashcraft/ and writes ui/kit/; `SMASHCRAFT_KIT=DIR` uses a kit you already
  have): the Replays page's simulation `sim.js`, its types, `viewer.lua` and
  test fixtures;
- at run time, the match records and replays the map writes into
  CustomMapData, and the maps in Warcraft III's Maps folders;
- for Online only, Smashcraft's tools (`SMASHCRAFT_TS`).

To move to a newer Smashcraft, put its commit in `smashcraft.ref`.

## Play

Play starts Warcraft III on the current map, the highest
`Smashcraft 0.0.N.w3x` directly in `Maps/00-Smashcraft` beside the History
page's folders, with Warcraft's own `-launch -loadfile MAP`
(src-tauri/src/play.rs). Warcraft III is `WARCRAFT_III` when set (any program
taking those arguments, such as a Proton launch script), else on Windows its
default install under Program Files, else on Linux the install in the Wine
prefix that holds the map, run with `wine` (`WINE`) and that `WINEPREFIX`.

## Controller support

Controller support is optional: Smashcraft plays on the keyboard alone, and
the service turns a pad into the same keys. The client never reads the pad itself. It connects to the Warcraft III
Controller service ([wc3-controller](https://github.com/tompassarelli/wc3-controller),
with Smashcraft's plug-in from smashcraft:controller) on 127.0.0.1:47631 and
shows its status, live input and bindings; the messages and plain-language
status rows come from wc3-controller's model crate, pinned by tag in
src-tauri/Cargo.toml. When nothing answers and the player has
turned controller support on (remembered in the client's settings), it starts
the bundled service with `--service --plugin wc3-journal`:
`WC3_CONTROLLER_SERVICE`, else `wc3-controller` beside the client, else on the
`PATH`; the plug-in is `wc3-journal` beside the client, else the one
Smashcraft's `bun wisp controller` installs; without one the service runs on
Any map. Releases bundle the service, not the plug-in. A started service gets 10 s
to answer, three starts in a row, then the page says it couldn't start. The
service is single-instance, so a client never starts a second copy beside a
running one. `WC3_CONTROLLER_PORT` points the client at a test service.

Closing the window hides it to the tray; the tray's light is the overall
status and its menu opens the window or quits. Opening Smashcraft again while it
runs shows the running window (the first client holds 127.0.0.1:47632). "Start with my computer"
starts the client hidden in the tray at login (Linux: an XDG autostart entry
naming the installed launcher; Windows: the autostart plugin).

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
ingest and stats are in ui/src/records.ts and
ui/src/stats.ts; the Rust side
(src-tauri/src/records.rs) only reads folders and keeps the
store. Fixture folders: ui/test/fixtures/records/.

## Online

The Online page hosts a private Battle.net game and shows its join code, or
joins one by code (smashcraft:docs/design/client.md, "Direct play"). It runs
`bun wisp online host` or `join CODE` in Smashcraft's tools (`SMASHCRAFT_TS`,
a Smashcraft checkout's ts/ with its dependencies installed) and shows their
output lines; their error output goes to `online.log` in the
client's log folder. Start now sends a waiting host `start` on its input, and
Cancel stops the run (a hosted game stays open until the player leaves it).
The first visit asks once to add Smashcraft's page to Warcraft III's menus
(`online setup`); the answer is remembered in the client's settings. The
runner is src-tauri/src/online.rs and the page
ui/src/pages/online.ts.

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
version that recorded the replay. The page build copies the kit's `sim.js`,
stamped with its source version, and the client keeps a copy in `sims/` for
every version it runs.

For any other version the client looks in the Maps folder beside each
CustomMapData folder (Download included) for a Smashcraft map stamped with
it. It reads that map's `war3map.lua` (src-tauri/src/mpq.rs),
keeps it as `sims/<version>.lua`, and plays the replay in the map's own
simulation: Lua 5.3.6 built with 32-bit numbers, as Warcraft's
(src-tauri/lua-5.3.6), with `viewer.lua`'s modules added
(src-tauri/src/mapsim.rs and mapsim.lua;
smashcraft:ts/src/game/replay/viewerDriver.ts). A replay whose version has
neither names it and asks for its map. Joining and opening are in ui/src/replays.ts
and ui/src/playback.ts. The Rust side
(src-tauri/src/replays.rs) only reads and keeps files.

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
rematch included (src-tauri/src/warcraft_replays.rs).

## Build, test, install

```sh
bun scripts/kit.ts       # ui/kit/ from the Smashcraft commit in smashcraft.ref
cd ui && bun install && bun test && bun test ../scripts && bun run check && bun run build
cd ../src-tauri && nix-shell ../shell.nix --run 'cargo test --jobs 2'
# Install for this user (NixOS): build, keep its libraries alive, write the
# launcher, desktop entry and refresh an enabled autostart entry.
cd .. && bun scripts/install.ts
```

The install finds cargo itself (scripts/cargo.ts): `CARGO`,
cargo on `PATH`, rustup's pinned toolchain in ~/.rustup, or a rustup in the
nix store running that channel. Outside nix-shell it builds inside shell.nix.
For `cargo test`, put rustup's 1.96.1 toolchain on `PATH` first
(`$HOME/.rustup/toolchains/1.96.1-x86_64-unknown-linux-gnu/bin`). The install
lives in ~/.local/share/smashcraft-build-inputs/smashcraft-client (one folder
per commit, `current`, the `smashcraft` launcher and a `runtime` out-link), never
in the system configuration; its launcher sets `SMASHCRAFT_TS` to
~/code/smashcraft/main/ts when that exists.

CI (.github/workflows/ci.yml) builds and tests on Linux and Windows and
uploads each build with the wc3-controller service of the tag pinned in
src-tauri/Cargo.toml. Pushing a tag `vX.Y.Z` (`safe-push --tag vX.Y.Z`)
attaches both to that release.
