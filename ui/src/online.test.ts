import { expect, test } from "bun:test";
import type { OnlineMode } from "./controller";
import { DID_NOT_START, looksLikeCode, looksLikeRelayCode, type Network, networkView, onlineView, SMASHCRAFT_SERVERS_OPT_IN, standaloneView, WAITING_FOR_OPPONENT } from "./online";

/** mulberry32: the same cases on every run. */
const random = (seed: number) => () => {
  seed = (seed + 0x6d2b79f5) | 0;
  let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
  t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
};

test("a run's view: only a host shows its join code, may start now only while running with a code and before the match loads, and a run fails only when it finished badly (seed 6, 2000 cases)", () => {
  const next = random(6);
  const pick = <T>(xs: readonly T[]) => xs[Math.floor(next() * xs.length)]!;
  const LINES = ["Join code: ABCD-EFGH", "Join code: WXYZ-1234", "Waiting for your opponent", "Starting the match", "Loading the match", "In the match", "Couldn't join that game. Check the code with your opponent."];
  for (let i = 0; i < 2000; i++) {
    const mode = pick<OnlineMode | null>(["host", "join", "setup", null]);
    const lines = Array.from({ length: Math.floor(next() * 5) }, () => pick(LINES));
    const running = next() < 0.5;
    const finished = pick([null, true, false]);
    const view = onlineView({ available: true, running, map: null, mode, lines, finished });
    const codes = lines.filter((line) => line.startsWith("Join code: ")).map((line) => line.slice("Join code: ".length));
    expect(view.code).toBe(mode === "host" ? codes[0] : undefined);
    expect(view.steps).toEqual(lines.filter((line) => !line.startsWith("Join code: ")));
    expect(view.canStartNow).toBe(mode === "host" && running && codes.length > 0 && !lines.includes("Loading the match"));
    expect(view.busy).toBe(running);
    expect(view.failed).toBe(finished === false);
  }
});

test("a typed code is sent exactly when it has eight characters besides spaces and dashes (seed 7, 2000 cases)", () => {
  const next = random(7);
  const ALNUM = "ABCDEFGHJKLMNPQRSTUVWXYZabcdefgh0123456789";
  for (let i = 0; i < 2000; i++) {
    const count = Math.floor(next() * 12);
    let typed = "";
    for (let c = 0; c < count; c++) {
      while (next() < 0.2) typed += next() < 0.5 ? "-" : " ";
      typed += ALNUM[Math.floor(next() * ALNUM.length)];
    }
    expect(looksLikeCode(typed)).toBe(count === 8);
  }
});

test("a standalone run's view reads the host's relay code and the region's round trip from the lines wisp play --standalone prints, and only a host shows the code (lines recorded in the gex45 two-client run of 30 October 2026)", () => {
  const lines = [
    "net: hosting through sgp, round trip to it 183 ms",
    "Smashcraft: join code sgp-7K2M-QX9D-R3TB-4 (relay room; the joiner's map must match 4abc9ceb)",
    "Smashcraft: standalone window opened. Connect a controller or use the keyboard.",
  ];
  const waiting = standaloneView({ available: true, map: null, running: true, mode: "host", lines: lines.slice(0, 2), finished: null });
  expect(waiting.steps).toEqual([WAITING_FOR_OPPONENT]);
  const host = standaloneView({ available: true, map: null, running: true, mode: "host", lines, finished: null });
  expect(host.code).toBe("sgp-7K2M-QX9D-R3TB-4");
  expect(host.region).toEqual({ name: "sgp", rttMs: 183 });
  expect(host.steps).toEqual(["standalone window opened. Connect a controller or use the keyboard."]);
  expect(standaloneView({ available: true, map: null, running: false, mode: "join", lines: [], finished: false }).steps).toEqual([DID_NOT_START]);
  expect(host.canStartNow).toBe(false);
  expect(host.busy).toBe(true);
  const joiner = standaloneView({ available: true, map: null, running: true, mode: "join", lines: ["net: joining through sgp, round trip to it 40 ms", lines[2]!], finished: false });
  expect(joiner.code).toBeUndefined();
  expect(joiner.region).toEqual({ name: "sgp", rttMs: 40 });
  expect(joiner.failed).toBe(true);
  expect(standaloneView({ available: true, map: null, running: true, mode: "join", lines: lines.slice(0, 2), finished: null }).code).toBeUndefined();
});

test("a typed relay code is sent when it names a region and thirteen letters or digits, or a relay's address and port with them (external reference: wisp:host/net/joinCode.ts relayCode)", () => {
  for (const good of ["sgp-7K2M-QX9D-R3TB-4", " sgp-7k2mqx9dr3tb4 ", "gex-0000-0000-0000-1", "159.89.205.60:7457/7K2M-QX9D-R3TB-4"]) expect(looksLikeRelayCode(good)).toBe(true);
  for (const bad of ["", "ABCD-EFGH", "sgp-7K2M-QX9D-R3TB", "sgp-7K2M-QX9D-R3TB-45", "7K2M-QX9D-R3TB-4", "R2M6-WDYR-7MQC-S0ZA", "sgp 7K2M QX9D R3TB 4"]) expect(looksLikeRelayCode(bad)).toBe(false);
});

test("Smashcraft Servers is off until the player opts in: picking it before then keeps Battle.net and asks on the opt-in screen (seed 8, 2000 cases)", () => {
  const next = random(8);
  const pick = <T>(xs: readonly T[]) => xs[Math.floor(next() * xs.length)]!;
  for (let i = 0; i < 2000; i++) {
    const picked = pick<Network>(["standalone", "battle.net", "smashcraft_servers"]);
    const optedIn = pick([null, false, true]);
    const view = networkView(picked, optedIn);
    expect(view.network).toBe(picked === "standalone" ? "standalone" : picked === "smashcraft_servers" && optedIn === true ? "smashcraft_servers" : "battle.net");
    expect(view.askOptIn).toBe(picked === "smashcraft_servers" && optedIn !== true);
  }
});

test("the opt-in screen's text is Tom's, word for word (decision, 25 October 2026)", () => {
  expect(SMASHCRAFT_SERVERS_OPT_IN).toBe("Smashcraft changes how Warcraft connects while it runs and Blizzard has not approved it.");
});
