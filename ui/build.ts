// Builds the client's pages into dist/, which the Rust app embeds.
import { cpSync, mkdirSync, rmSync } from "node:fs";

const here = new URL(".", import.meta.url).pathname;
rmSync(here + "dist", { recursive: true, force: true });
mkdirSync(here + "dist");
const result = await Bun.build({
  entrypoints: [here + "src/main.ts"],
  outdir: here + "dist",
  target: "browser",
  minify: true,
});
if (!result.success) {
  for (const log of result.logs) console.error(log);
  process.exit(1);
}
cpSync(here + "index.html", here + "dist/index.html");
cpSync(here + "styles.css", here + "dist/styles.css");
