// Finds the cargo that builds the client: CARGO, cargo on PATH, the pinned
// toolchain rustup installed (~/.rustup/toolchains/<channel>-<host>), or a
// rustup from the nix store running that channel. The toolchain's own folder
// goes first on PATH so cargo finds its rustc.
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";

export type Cargo = {
  /** The command that runs cargo. */
  command: string[];
  /** A folder to put first on PATH, so cargo finds the toolchain's rustc. */
  path?: string;
};

export type CargoSearch = {
  env: Record<string, string | undefined>;
  home: string;
  /** The channel the client pins (src-tauri/rust-toolchain.toml). */
  channel: string;
  /** Host triple suffixes rustup names its toolchains with. */
  hosts: readonly string[];
  /** Folders holding nix store paths. */
  stores: readonly string[];
  exists: (path: string) => boolean;
  list: (dir: string) => string[];
  which: (command: string) => string | null;
};

/** The channel in a rust-toolchain.toml. */
export function pinnedChannel(toml: string): string | undefined {
  return /^\s*channel\s*=\s*"([^"]+)"/m.exec(toml)?.[1];
}

export function findCargo(search: CargoSearch): Cargo | undefined {
  const given = search.env.CARGO;
  if (given !== undefined && search.exists(given)) return { command: [given], path: dirname(given) };
  const onPath = search.which("cargo");
  if (onPath !== null) return { command: [onPath] };
  const rustupHome = search.env.RUSTUP_HOME ?? join(search.home, ".rustup");
  for (const host of search.hosts) {
    const bin = join(rustupHome, "toolchains", `${search.channel}-${host}`, "bin");
    if (search.exists(join(bin, "cargo"))) return { command: [join(bin, "cargo")], path: bin };
  }
  const rustup = search.which("rustup") ?? search.stores.flatMap((store) =>
    search.list(store).filter((name) => /^[a-z0-9]{32}-rustup-[0-9.]+$/.test(name)).sort().map((name) => join(store, name, "bin/rustup")),
  ).find((path) => search.exists(path));
  return rustup === undefined || rustup === null ? undefined : { command: [rustup, "run", search.channel, "cargo"] };
}

const HOSTS: Record<string, string> = {
  "linux-x64": "x86_64-unknown-linux-gnu", "linux-arm64": "aarch64-unknown-linux-gnu",
  "darwin-x64": "x86_64-apple-darwin", "darwin-arm64": "aarch64-apple-darwin", "win32-x64": "x86_64-pc-windows-msvc",
};

/** This machine's search, for the client at `client`. */
export function hostSearch(client: string): CargoSearch {
  const channel = pinnedChannel(readFileSync(join(client, "src-tauri/rust-toolchain.toml"), "utf8")) ?? "stable";
  const host = HOSTS[`${process.platform}-${process.arch}`];
  return {
    env: process.env, home: homedir(), channel, hosts: host === undefined ? [] : [host], stores: ["/nix/store"],
    exists: existsSync,
    list: (dir) => {
      try {
        return readdirSync(dir);
      } catch {
        return [];
      }
    },
    which: (command) => Bun.which(command),
  };
}
