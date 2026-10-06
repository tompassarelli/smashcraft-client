# Smashcraft client

The desktop app players open: a window with routed pages (Controller now;
history, stats, replays and online slot into `ROUTES` in
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

## Build, test, install

```sh
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
