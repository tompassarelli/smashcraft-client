import { expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const RUST = join(import.meta.dir, "../../src-tauri/src");
const COMMANDS = join(RUST, "commands");
const read = (path: string) => readFileSync(path, "utf8");

const rustFiles = (dir: string): string[] =>
  readdirSync(dir, { withFileTypes: true }).flatMap((entry) => (entry.isDirectory() ? rustFiles(join(dir, entry.name)) : entry.name.endsWith(".rs") ? [join(dir, entry.name)] : []));

const names = (text: string, pattern: RegExp) => [...text.matchAll(pattern)].map((match) => match[1]!);

const camel = (snake: string) => snake.replace(/_(\w)/g, (_, letter: string) => letter.toUpperCase());

/** Splits a parameter list at the commas outside generics, parentheses and brackets. */
function parameters(list: string): string[] {
  const out: string[] = [];
  let depth = 0;
  let current = "";
  for (const char of list) {
    if ("<([".includes(char)) depth++;
    if (">)]".includes(char)) depth--;
    if (char === "," && depth === 0) {
      out.push(current);
      current = "";
    } else current += char;
  }
  if (current.trim() !== "") out.push(current);
  return out.map((parameter) => parameter.trim());
}

const INJECTED = /^(State|AppHandle|Window|WebviewWindow)\b/;

/** Each registered command's name and the argument names Tauri takes from the UI (camelCase of its own parameters). */
function registered() {
  const handler = read(join(COMMANDS, "mod.rs")).match(/generate_handler!\[([^\]]*)\]/)![1]!;
  const sources = new Map(readdirSync(COMMANDS).map((file) => [file.replace(/\.rs$/, ""), read(join(COMMANDS, file))]));
  return names(handler, /(\w+::\w+)/g).map((path) => {
    const [module, command] = path.split("::") as [string, string];
    const signature = new RegExp(`#\\[tauri::command\\]\\s*pub (?:async )?fn ${command}\\(([^{]*?)\\)\\s*(?:->|\\{)`).exec(sources.get(module) ?? "");
    if (!signature) throw new Error(`${path} is in generate_handler! but is not a #[tauri::command] pub fn in commands/${module}.rs`);
    const args = parameters(signature[1]!)
      .filter((parameter) => !INJECTED.test(parameter.slice(parameter.indexOf(":") + 1).trim()))
      .map((parameter) => camel(parameter.slice(0, parameter.indexOf(":")).trim()));
    return { command, args };
  });
}

/** Each command the UI invokes with the argument names it sends. */
function invoked() {
  const calls = ["api.ts", "tauri-online.ts"].flatMap((file) => [...read(join(import.meta.dir, file)).matchAll(/invoke<[^>]*(?:<[^>]*>[^>]*)*>\("(\w+)"(?:, \{([^}]*)\})?\)/g)]);
  return calls.map((call) => ({ command: call[1]!, args: (call[2] ?? "").split(",").map((arg) => arg.trim().split(":")[0]!.trim()).filter((arg) => arg !== "") }));
}

test("the commands generate_handler! registers are the commands ui/src/api.ts and tauri-online.ts invoke, with the same argument names (integration check of the Rust-to-TypeScript boundary)", () => {
  const rust = registered();
  const ui = invoked();
  expect(rust.length).toBeGreaterThan(30);
  expect(ui.map((call) => call.command).sort()).toEqual(rust.map((command) => command.command).sort());
  for (const call of ui) expect({ command: call.command, args: [...call.args].sort() }).toEqual({ command: call.command, args: [...rust.find((command) => command.command === call.command)!.args].sort() });
});

test("the events Rust emits are the events the UI listens for (integration check of the Rust-to-TypeScript boundary)", () => {
  const emitted = rustFiles(RUST).flatMap((file) => names(read(file), /\.emit\("(\w+)"/g));
  const listened = ["api.ts", "tauri-online.ts"].flatMap((file) => names(read(join(import.meta.dir, file)), /listen<[^>]*>\("(\w+)"/g));
  expect(emitted.length).toBeGreaterThan(0);
  expect([...new Set(listened)].sort()).toEqual([...new Set(emitted)].sort());
});
