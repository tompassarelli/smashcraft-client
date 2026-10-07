import { expect, test } from "bun:test";
import { join } from "node:path";
import { type CargoSearch, findCargo, pinnedChannel } from "./cargo";

const search = (files: string[], onPath: Record<string, string> = {}, env: Record<string, string> = {}): CargoSearch => ({
  env, home: "/home/p", channel: "1.96.1", hosts: ["x86_64-unknown-linux-gnu"], stores: ["/nix/store"],
  exists: (path) => files.includes(path),
  list: (dir) => files.filter((f) => f.startsWith(`${dir}/`)).map((f) => f.slice(dir.length + 1).split("/")[0]!),
  which: (command) => onPath[command] ?? null,
});

test("cargo on PATH wins; CARGO wins over it", () => {
  expect(findCargo(search([], { cargo: "/usr/bin/cargo" }))).toEqual({ command: ["/usr/bin/cargo"] });
  expect(findCargo(search(["/opt/rust/bin/cargo"], { cargo: "/usr/bin/cargo" }, { CARGO: "/opt/rust/bin/cargo" }))).toEqual({ command: ["/opt/rust/bin/cargo"], path: "/opt/rust/bin" });
});

test("without cargo on PATH, the pinned toolchain rustup installed is used with its folder first on PATH", () => {
  const bin = join("/home/p", ".rustup", "toolchains", "1.96.1-x86_64-unknown-linux-gnu", "bin");
  expect(findCargo(search([join(bin, "cargo")]))).toEqual({ command: [join(bin, "cargo")], path: bin });
});

test("with only a rustup in the nix store, it runs the pinned channel's cargo", () => {
  const rustup = join("/nix/store", "90ljbla2hjxx5w372fcwsl82i5pvn771-rustup-1.29.0", "bin/rustup");
  expect(findCargo(search([`/nix/store/90ljbla2hjxx5w372fcwsl82i5pvn771-rustup-1.29.0/bin/rustup`, rustup, "/nix/store/hjakvji1fin5darwqm63fpr5skhff071-rustup-1.29.0-vendor.drv"]))).toEqual({ command: [rustup, "run", "1.96.1", "cargo"] });
  expect(findCargo(search([]))).toBeUndefined();
});

test("the pinned channel is read from rust-toolchain.toml", () => {
  expect(pinnedChannel('[toolchain]\nchannel = "1.96.1"\nprofile = "minimal"\n')).toBe("1.96.1");
});
