import { expect, test } from "bun:test";
import type { OnlineMode } from "./model";
import { looksLikeCode, type Network, networkView, onlineView, SMASHCRAFT_SERVERS_OPT_IN } from "./online";

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
    const view = onlineView({ available: true, running, mode, lines, finished });
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

test("Smashcraft Servers is off until the player opts in: picking it before then keeps Battle.net and asks on the opt-in screen (seed 8, 2000 cases)", () => {
  const next = random(8);
  const pick = <T>(xs: readonly T[]) => xs[Math.floor(next() * xs.length)]!;
  for (let i = 0; i < 2000; i++) {
    const picked = pick<Network>(["battle.net", "smashcraft_servers"]);
    const optedIn = pick([null, false, true]);
    const view = networkView(picked, optedIn);
    expect(view.network).toBe(picked === "smashcraft_servers" && optedIn === true ? "smashcraft_servers" : "battle.net");
    expect(view.askOptIn).toBe(picked === "smashcraft_servers" && optedIn !== true);
  }
});

test("the opt-in screen's text is Tom's, word for word (decision, 25 October 2026)", () => {
  expect(SMASHCRAFT_SERVERS_OPT_IN).toBe("Smashcraft changes how Warcraft connects while it runs and Blizzard has not approved it.");
});
