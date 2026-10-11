import { expect, test } from "bun:test";
import { dirname, join } from "node:path";
import { type CargoSearch, findCargo, pinnedChannel } from "./cargo";

/** mulberry32: the same cases on every run. */
const random = (seed: number) => () => {
  seed = (seed + 0x6d2b79f5) | 0;
  let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
  t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
};

test("cargo is found in the documented order: CARGO, PATH, the pinned rustup toolchain, then a rustup running that channel (seed 1, 500 cases)", () => {
  const next = random(1);
  const coin = () => next() < 0.5;
  const pick = <T>(xs: readonly T[]) => xs[Math.floor(next() * xs.length)]!;
  const home = "/home/p";
  const hosts = ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"];
  for (let i = 0; i < 500; i++) {
    const channel = pick(["1.96.1", "1.80.0", "nightly-2026-01-01"]);
    const files: string[] = [];
    const env: Record<string, string> = {};
    const onPath: Record<string, string> = {};
    const cargoEnv = "/opt/rust/bin/cargo";
    if (coin()) env.CARGO = cargoEnv;
    if (coin()) files.push(cargoEnv);
    if (coin()) onPath.cargo = "/usr/bin/cargo";
    const host = pick(hosts);
    const toolchain = join(home, ".rustup", "toolchains", `${pick([channel, "1.70.0"])}-${host}`, "bin");
    if (coin()) files.push(join(toolchain, "cargo"));
    if (coin()) onPath.rustup = "/usr/bin/rustup";
    const storeEntry = "/nix/store/90ljbla2hjxx5w372fcwsl82i5pvn771-rustup-1.29.0";
    const storeRustup = join(storeEntry, "bin/rustup");
    if (coin()) files.push(`${storeEntry}/bin/rustup`, storeRustup);
    if (coin()) files.push("/nix/store/hjakvji1fin5darwqm63fpr5skhff071-rustup-1.29.0-vendor.drv");
    const search: CargoSearch = {
      env, home, channel, hosts, stores: ["/nix/store"],
      exists: (path) => files.includes(path),
      list: (dir) => files.filter((f) => f.startsWith(`${dir}/`)).map((f) => f.slice(dir.length + 1).split("/")[0]!),
      which: (command) => onPath[command] ?? null,
    };
    const pinned = join(home, ".rustup", "toolchains", `${channel}-${host}`, "bin");
    const expected =
      env.CARGO !== undefined && files.includes(cargoEnv) ? { command: [cargoEnv], path: dirname(cargoEnv) }
      : onPath.cargo !== undefined ? { command: [onPath.cargo] }
      : files.includes(join(pinned, "cargo")) ? { command: [join(pinned, "cargo")], path: pinned }
      : onPath.rustup !== undefined ? { command: [onPath.rustup, "run", channel, "cargo"] }
      : files.includes(storeRustup) ? { command: [storeRustup, "run", channel, "cargo"] }
      : undefined;
    expect(findCargo(search)).toEqual(expected);
  }
});

test("the pinned channel round-trips through rust-toolchain.toml (seed 2, 200 cases)", () => {
  const next = random(2);
  for (let i = 0; i < 200; i++) {
    const channel = `1.${Math.floor(next() * 200)}.${Math.floor(next() * 10)}${next() < 0.3 ? "-beta" : ""}`;
    const before = next() < 0.5 ? "[toolchain]\n" : "";
    const after = next() < 0.5 ? 'profile = "minimal"\ncomponents = ["rustfmt"]\n' : "";
    expect(pinnedChannel(`${before}${next() < 0.5 ? "  " : ""}channel ${next() < 0.5 ? "=" : " = "} "${channel}"\n${after}`)).toBe(channel);
  }
});
