# Architecture: where code lives

The client is a Tauri tray app: it supervises padbridge, shows controller settings, lists match records and replays, replays sims through the map's own Lua, and launches Warcraft and online play.
It owns no game rule and no controller contract: it consumes padbridge's protocol types and Smashcraft's client kit.
Paths are repo-relative; the tables say where things are today, and "moves in Cn" names the slice that relocates them (last section).

## Modules and use direction

A module may use only the modules in its "may use" column. The UI talks to Rust only through `ui/src/api.ts` and, for the Online page, `ui/src/tauri-online.ts`.

| module | today | responsibility | may use |
|---|---|---|---|
| link | `src-tauri/src/link/`: mod (service port), supervisor, client | Start and watch padbridge and its plug-in; TCP client | padbridge protocol crate only |
| game-files | `src-tauri/src/`: records, replays, warcraft_replays, mapsim, mpq, lua32; `src-tauri/src/game/map_facts.rs` | Read the map's records, replays and Lua through the kit; keep `LastReplay.w3g` copies | none |
| launch | `src-tauri/src/`: play, online; `src-tauri/src/launch/`: relay, lan_switch | Start Warcraft, `bun wisp online`, `bun wisp play --standalone` and relay matches on Smashcraft Servers | game-files, servers |
| app | `src-tauri/src/lib.rs` (setup); `commands/` (handlers by page and the command list); sink (tray light and menu); settings, tray, autostart, single | Tauri wiring and command handlers | link, game-files, launch |
| ui-types | `ui/src/controller.ts` (hand mirror) | Protocol and command types | none |
| ui-core | `ui/src/`: tauri, tauri-online, dom, draw, playback, stats, history, online, play, records, replays, router | Invoke wrappers and pure helpers | ui-types |
| ui-pages | `ui/src/pages/`, `ui/src/main.ts` | One page per tab | ui-core, ui-types |
| scripts | `scripts/` | Kit fetch, install, cargo, icons | none |
| servers | `src-tauri/servers/` | Stand-in for the private Smashcraft Servers crate (feature `smashcraft-servers`) | none |

`src-tauri/lua-5.3.6/` is unmodified vendored Lua: never edit it.

## Where things live

| change | edit today | target |
|---|---|---|
| A key, binding or fighter action | not here: padbridge `model/src/lib.rs`; UI mirror `ui/src/controller.ts` (`KEYS`, `Press`) | Smashcraft `keyBindings.ts` (K1/P1); `ui/src/generated/protocol.ts` from padbridge's Rust types in C1 |
| A new pad preset | labels in `src-tauri/src/lib.rs` (`pad_presets`), `ui/src/controller.ts` (`PadPreset`) | read from the snapshot's layout list in C1 |
| Service TCP protocol types | crate `padbridge-model` v0.4.0 (`src-tauri/Cargo.toml`), `ui/src/controller.ts` | padbridge `protocol` crate by tag + generated `ui/src/generated/protocol.ts` in C1 |
| Service supervision and TCP client | `src-tauri/src/link/{supervisor,client}.rs` (`WC3_CONTROLLER_*`, `padbridge` binary) | `PADBRIDGE_*`, `padbridge` in C1 |
| Plug-in protocol | not here: padbridge `src/service/plugin.rs`; the client only passes `--plugin` (`plugin_program` in `src-tauri/src/link/client.rs`) | padbridge `protocol/src/plugin.rs` in P1 |
| Controller page | `ui/src/pages/controller.ts` (face layout hardcoded), commands in `src-tauri/src/lib.rs` | face layout from the snapshot in C1; `src-tauri/src/commands/controller.rs` in C3 |
| Records parsing and history | `ui/src/records.ts`, `ui/src/history.ts`, folders in `src-tauri/src/records.rs` | kit `records.js` in C2; `game/records.rs` in C3 |
| Map replays and the viewer sim | `src-tauri/src/replays.rs`, `src-tauri/src/mapsim.rs`, `src-tauri/src/mapsim.lua`, `ui/src/replays.ts`, `ui/src/pages/replays.ts` | kit `viewerEntry`/`versionModule` in C2; `game/{map_replays,mapsim}.rs` in C3 |
| Warcraft's `LastReplay.w3g` library | `src-tauri/src/warcraft_replays.rs` | `game/w3g_library.rs` in C3 |
| Record, replay and map file names | `src-tauri/src/records.rs`, `src-tauri/src/replays.rs`, `src-tauri/src/play.rs` | kit `files.json` in C2 |
| Launching Warcraft | `src-tauri/src/play.rs`, `ui/src/play.ts` | `launch/warcraft.rs` in C3 |
| Smashcraft Servers (relay Host and Join) | `src-tauri/src/launch/relay.rs` (runs wisp's `AgentHost`), `launch/lan_switch.rs` (the switch's interface only), `src-tauri/src/game/map_facts.rs`, `ui/src/pages/online.ts` | stays |
| Online play | `src-tauri/src/online.rs` (argv per smashcraft `docs/client-interface.md` and `docs/commands/online.md`; the standalone window's run is its `standalone_args`), `ui/src/online.ts`, `ui/src/pages/online.ts` | `launch/online.rs` in C3; argv in kit `files.json` in C2 |
| A new page | `ui/src/router.ts` (`ROUTES`), `ui/src/main.ts`, `ui/src/pages/` | stays; its commands in `commands/<page>.rs` in C3 |
| Smashcraft version and kit | `smashcraft.ref`, `scripts/kit.ts` | a release tag or unrewritten main SHA; kit 2 in C2 |
| Desktop entry | `src-tauri/src/autostart.rs`, duplicated in `scripts/install.ts` | `install.ts` calls `--print-desktop-entry` (no slice yet) |
| Single instance, tray, settings | `src-tauri/src/single.rs`, `src-tauri/src/tray.rs`, `src-tauri/src/settings.rs` | `single.rs` -> `instance_lock.rs` in C3 |

Tests sit beside their code: `ui/src/*.test.ts`, `scripts/*.test.ts`, Rust `#[cfg(test)]`; fixtures in `ui/test/fixtures/` and `src-tauri/tests/fixtures/`.

## What it consumes, from whom

| input | owner | target channel |
|---|---|---|
| Service, plug-in, settings, status types; face positions; preset list | padbridge `protocol` | crate pinned by tag; TS generated by ts-rs (`src-tauri/tests/export_bindings.rs`, run by `bun scripts/generate.ts`) |
| Viewer sim, records parser, file names, map install path | Smashcraft client kit (`ts/scripts/clientKit.ts`) | kit 2: `kit.json` (`viewerEntry`, `versionModule`), `records.js`, `files.json` |
| Online play | Smashcraft `bun wisp online` and `bun wisp play --standalone --host --relay REGION` / `--join CODE` | unchanged argv, documented in kit `files.json` |
| Relay agent and room protocol | wisp `relay-agent`, `relay-proto` | crates pinned to a wisp main SHA |

## Target and slices

- `src-tauri/src/`: `app.rs`, `commands/{controller,records,replays,sims,play,online}.rs`, `link/`, `game/`, `launch/`, `instance_lock.rs`.
- `ui/src/generated/` holds produced types only (from `scripts/generate.ts`); `ui/src/controller.ts` disappears.
- Each slice moves files and every caller in one change, with no re-export shims.

1. C1 (after padbridge P1): pin padbridge `protocol` by tag; `WC3_CONTROLLER_*` -> `PADBRIDGE_*`; generated protocol.ts replaces controller.ts; presets and face layout from the snapshot.
2. C2 (with smashcraft `clientKit.ts`): kit 2; `mapsim.rs` reads `viewerEntry`/`versionModule`; `ui/src/records.ts` replaced by kit `records.js`.
3. C3: `lib.rs` split into `app.rs` + `commands/`; `link.rs` split; the renames above.

Checks: `bun scripts/cargo.ts test`, `bun test ui`, a `--check` start.
Payoff: a controller feature touches padbridge `protocol` plus one client page, with no paired pin and mirror edits.
