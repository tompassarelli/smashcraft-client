// Puts Smashcraft's client kit (smashcraft:docs/client-interface.md) in ui/kit/.
// SMASHCRAFT_KIT names a kit folder to copy; otherwise the kit is written by the
// Smashcraft commit in smashcraft.ref, fetched (ts/ only) into .smashcraft/<commit>.
//   bun scripts/kit.ts
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const kit = join(root, "ui/kit");
const KIT = 1;

function run(command: string[], cwd: string) {
  const result = Bun.spawnSync(command, { cwd, stdout: "inherit", stderr: "inherit" });
  if (result.exitCode !== 0) throw new Error(`${command.join(" ")} failed (exit ${result.exitCode})`);
}

function check() {
  const manifest = JSON.parse(readFileSync(join(kit, "kit.json"), "utf8")) as { kit: number; version: string };
  if (manifest.kit !== KIT) throw new Error(`the kit is kit ${manifest.kit}; this client reads kit ${KIT}`);
  console.log(`client kit ${manifest.kit}, Smashcraft version ${manifest.version}`);
}

const given = process.env.SMASHCRAFT_KIT;
if (given !== undefined && given !== "") {
  rmSync(kit, { recursive: true, force: true });
  cpSync(given, kit, { recursive: true });
} else {
  const commit = readFileSync(join(root, "smashcraft.ref"), "utf8").trim();
  if (!/^[0-9a-f]{40}$/.test(commit)) throw new Error("smashcraft.ref must hold one full commit id");
  const stamp = join(kit, "commit");
  if (existsSync(stamp) && readFileSync(stamp, "utf8").trim() === commit) {
    check();
    process.exit(0);
  }
  const source = join(root, ".smashcraft", commit);
  if (!existsSync(join(source, "ts/scripts/clientKit.ts"))) {
    mkdirSync(source, { recursive: true });
    run(["git", "init", "-q"], source);
    run(["git", "sparse-checkout", "set", "ts"], source);
    run(["git", "fetch", "-q", "--depth", "1", "--filter=blob:none", "https://github.com/tompassarelli/smashcraft", commit], source);
    run(["git", "checkout", "-q", "FETCH_HEAD"], source);
  }
  run(["bun", "install", "--frozen-lockfile", "--ignore-scripts"], join(source, "ts"));
  run(["bun", "scripts/clientKit.ts", kit], join(source, "ts"));
  await Bun.write(stamp, `${commit}\n`);
}
check();
