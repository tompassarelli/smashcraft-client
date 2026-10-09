// Builds the client's pages into dist/, which the Rust app embeds.
import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { SOURCE_STAMP_TEXT, sourceVersion } from "../../ts/scripts/sourceVersion";
import { viewerModules } from "../../ts/scripts/viewerLua";

const here = import.meta.dir;
const dist = join(here, "dist");
rmSync(dist, { recursive: true, force: true });
mkdirSync(dist);
const result = await Bun.build({
  entrypoints: [join(here, "src/main.ts")],
  outdir: dist,
  target: "browser",
  minify: true,
});
if (!result.success) {
  for (const log of result.logs) console.error(log);
  process.exit(1);
}
// The simulation the Replays page watches replays in: this source's version
// (smashcraft:ts/scripts/sourceVersion.ts), kept by the client for every
// version it has played.
const ts = join(here, "../../ts");
const sim = await Bun.build({
  entrypoints: [join(ts, "src/game/replay/viewerBundle.ts")],
  outdir: dist,
  naming: "sim.js",
  target: "browser",
  format: "esm",
  minify: true,
});
if (!sim.success) {
  for (const log of sim.logs) console.error(log);
  process.exit(1);
}
const simFile = join(dist, "sim.js");
const simCode = readFileSync(simFile, "utf8");
if (!simCode.includes(SOURCE_STAMP_TEXT)) throw new Error("sim.js holds no source stamp to replace");
writeFileSync(simFile, simCode.replaceAll(SOURCE_STAMP_TEXT, JSON.stringify(sourceVersion(ts))));
// The viewer's modules for playing a replay in its own map's simulation (ts/scripts/viewerLua.ts).
const tstl = Bun.spawnSync([process.execPath, "--bun", join(ts, "node_modules/typescript-to-lua/dist/tstl.js"), "-p", join(ts, "tsconfig.viewer-lua.json")], { cwd: ts, stdout: "inherit", stderr: "inherit" });
if (tstl.exitCode !== 0) process.exit(1);
writeFileSync(join(dist, "viewer.lua"), viewerModules(readFileSync(join(ts, "build/viewer-lua/viewer.lua"), "utf8")));
cpSync(join(here, "index.html"), join(dist, "index.html"));
cpSync(join(here, "styles.css"), join(dist, "styles.css"));
