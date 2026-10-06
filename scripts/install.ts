// Installs the client for this Linux user, outside any system configuration:
// ~/.local/share/smashcraft-build-inputs/smashcraft-client/<commit>/ holds the
// build, `current` points at it, `smashcraft` is the launcher the desktop entry
// and autostart run. Run from client/ inside its shell:
//   nix-shell shell.nix --run "bun scripts/install.ts"
import { $ } from "bun";
import { cpSync, existsSync, mkdirSync, renameSync, rmSync, symlinkSync, writeFileSync, chmodSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

const client = new URL("..", import.meta.url).pathname;
const home = homedir();
const root = join(home, ".local/share/smashcraft-build-inputs/smashcraft-client");
const commit = (await $`git -C ${client} rev-parse --short=12 HEAD`.text()).trim();
const dirty = (await $`git -C ${client} status --porcelain -- .`.text()).trim() !== "";
const version = dirty ? `${commit}-dirty` : commit;

await $`bun install --frozen-lockfile`.cwd(join(client, "ui"));
await $`bun run build`.cwd(join(client, "ui"));
await $`nice -n 10 cargo build --release --locked --jobs ${process.env.JOBS ?? "2"}`.cwd(join(client, "src-tauri"));

// Keep the libraries the binary links (and the tray library it loads at run time) alive.
const runtime = join(root, "runtime");
await $`nix-build ${join(client, "runtime.nix")} -o ${runtime}`.quiet();

const dir = join(root, version);
rmSync(dir, { recursive: true, force: true });
mkdirSync(dir, { recursive: true });
cpSync(join(client, "src-tauri/target/release/smashcraft"), join(dir, "smashcraft"));
cpSync(join(client, "src-tauri/icons/icon.png"), join(dir, "icon.png"));
const next = join(root, "current.next");
rmSync(next, { force: true });
symlinkSync(version, next);
renameSync(next, join(root, "current"));

const service = process.env.WC3_CONTROLLER_SERVICE ?? "";
const launcher = join(root, "smashcraft");
writeFileSync(
  launcher,
  `#!/bin/sh
# Smashcraft client launcher (written by smashcraft:client/scripts/install.ts).
export SMASHCRAFT_LAUNCHER="${launcher}"
export LD_LIBRARY_PATH="${runtime}/lib\${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
${service ? `export WC3_CONTROLLER_SERVICE="${service}"\n` : ""}exec "${root}/current/smashcraft" "$@"
`,
);
chmodSync(launcher, 0o755);

const applications = join(home, ".local/share/applications");
mkdirSync(applications, { recursive: true });
writeFileSync(
  join(applications, "smashcraft.desktop"),
  `[Desktop Entry]
Type=Application
Name=Smashcraft
Comment=Controller support and play for Smashcraft
Exec="${launcher}"
Icon=${root}/current/icon.png
Terminal=false
Categories=Game;
`,
);
// An autostart entry the player turned on keeps pointing at the launcher; refresh it.
const autostart = join(process.env.XDG_CONFIG_HOME ?? join(home, ".config"), "autostart/smashcraft.desktop");
if (existsSync(autostart)) {
  writeFileSync(
    autostart,
    `[Desktop Entry]\nType=Application\nName=Smashcraft\nComment=Controller support for Warcraft III, waiting in the tray\nExec="${launcher}" --hidden\nTerminal=false\nX-GNOME-Autostart-enabled=true\n`,
  );
}
console.log(`installed ${version}: ${launcher}`);
