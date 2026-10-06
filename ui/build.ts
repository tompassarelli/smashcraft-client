// Builds the client's pages into dist/, which the Rust app embeds.
import { cpSync, mkdirSync, rmSync } from "node:fs";
import { join } from "node:path";

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
cpSync(join(here, "index.html"), join(dist, "index.html"));
cpSync(join(here, "styles.css"), join(dist, "styles.css"));
