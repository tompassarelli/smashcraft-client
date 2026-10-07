import { expect, test } from "bun:test";
import { looksLikeCode, onlineView } from "./online";

const state = (mode: "host" | "join" | "setup", lines: string[], running = true, finished: boolean | null = null) => ({ available: true, running, mode, lines, finished });

test("a host shows its code and may start now until the match loads", () => {
  const waiting = onlineView(state("host", ["Join code: ABCD-EFGH", "Waiting for your opponent"]));
  expect(waiting).toEqual({ code: "ABCD-EFGH", steps: ["Waiting for your opponent"], canStartNow: true, busy: true, failed: false });
  expect(onlineView(state("host", ["Join code: ABCD-EFGH", "Starting the match", "Loading the match"])).canStartNow).toBe(false);
  expect(onlineView(state("host", ["Join code: ABCD-EFGH", "In the match"], false, true)).canStartNow).toBe(false);
});

test("before Warcraft III creates the game there is no code to show", () => {
  expect(onlineView(state("host", [])).code).toBeUndefined();
  expect(onlineView(state("host", [])).canStartNow).toBe(false);
});

test("a guest never shows a code and a failed run reads as failed", () => {
  const failed = onlineView(state("join", ["Couldn't join that game. Check the code with your opponent."], false, false));
  expect(failed.code).toBeUndefined();
  expect(failed.canStartNow).toBe(false);
  expect(failed.failed).toBe(true);
});

test("codes are sent once they have eight characters", () => {
  expect(looksLikeCode("abcd-efgh")).toBe(true);
  expect(looksLikeCode("ABCD EFG")).toBe(false);
});
