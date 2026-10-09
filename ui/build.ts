// Builds the client's pages into dist/, which the Rust app embeds. Needs ui/kit/ (bun ../scripts/kit.ts).
import { cpSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { join } from "node:path";

const here = import.meta.dir;
const kit = join(here, "kit");
if (!existsSync(join(kit, "kit.json"))) throw new Error("no Smashcraft client kit in ui/kit: run bun ../scripts/kit.ts");
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
// The Replays page's own simulation and the viewer's Lua modules for older maps, from the kit.
cpSync(join(kit, "sim.js"), join(dist, "sim.js"));
cpSync(join(kit, "viewer.lua"), join(dist, "viewer.lua"));
cpSync(join(here, "index.html"), join(dist, "index.html"));
cpSync(join(here, "styles.css"), join(dist, "styles.css"));
