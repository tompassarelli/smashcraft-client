// #141: replays listed beside their records, joined from the map's parts,
// copied as one file and watched with pause, frame step and seek; a replay
// from a version the client doesn't hold names that version.
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { TAPE_REPLAY_SERIAL, recordTapeReplay } from "../../../ts/src/game/replay/tapeReplay";
import * as ownSimulation from "../../../ts/src/game/replay/viewer";
import { Playback, clock } from "./playback";
import { type ReplayFile, type Simulation, type Simulations, joinedReplay, keptName, openForWatching, replayEntries } from "./replays";

/** Lines as Warcraft writes a Preload file. */
const preload = (lines: readonly string[]) => `function PreloadFiles takes nothing returns nothing\n${lines.map((line) => `\tcall Preload( "${line}" )\n`).join("")}endfunction\n`;

const recorded = recordTapeReplay(700, 401);
const manifest = preload(recorded.manifest);
const parts = recorded.parts.map(preload);
const own: Simulation = ownSimulation;

const simulations = (kept: Record<string, Simulation> = {}): Simulations => ({
  own: async () => own,
  kept: async (version) => kept[version],
  held: async () => [own.sourceVersion(), ...Object.keys(kept)],
});

test("a manifest is listed with the record of the same match beside it; a stray file is refused", () => {
  const record = readFileSync(join(import.meta.dir, "../test/fixtures/records/a/smashcraft-match-1.txt"), "utf8").replace(/build=\S+/, "build=test").replace(/serial=\d+/, `serial=${TAPE_REPLAY_SERIAL}`);
  const files: ReplayFile[] = [
    { folder: "a", name: `smashcraft-replay-${TAPE_REPLAY_SERIAL}.txt`, text: manifest, modified: 2 },
    { folder: "b", name: "notes.txt", text: "hello", modified: 1 },
  ];
  const { entries, refused } = replayEntries(files, [{ folder: "a", name: `smashcraft-match-${TAPE_REPLAY_SERIAL}.txt`, text: record, modified: 2 }]);
  expect(entries.map((e) => [e.build, e.version, e.serial, e.frames, e.parts])).toEqual([["test", "development", TAPE_REPLAY_SERIAL, 700, parts.length]]);
  expect(entries[0]?.record?.serial).toBe(TAPE_REPLAY_SERIAL);
  expect(refused.map((r) => r.file)).toEqual(["notes.txt"]);
});

test("a replay joined from its parts and copied as one file plays with pause, frame step and seek", async () => {
  expect(joinedReplay(manifest, parts.slice(1))).toBe(`the replay has ${parts.length} parts; ${parts.length - 1} were found`);
  const joined = joinedReplay(manifest, parts);
  if (typeof joined === "string") throw new Error(joined);
  expect(keptName(joined)).toBe(`smashcraft-test-replay-${TAPE_REPLAY_SERIAL}-development.txt`);
  // Copied to another computer: the one file, opened on its own.
  const copied = joinedReplay(`${joined.join("\n")}\n`, []);
  if (typeof copied === "string") throw new Error(copied);
  const opened = await openForWatching(copied, simulations());
  if ("problem" in opened) throw new Error(opened.problem);
  const playback = new Playback(opened.viewer);
  const { viewer } = playback;
  expect([viewer.first, viewer.last, viewer.frame]).toEqual([0, 700, 0]);
  playback.toggle();
  expect(playback.tick(1000)).toBe(true);
  expect(viewer.frame).toBe(6);
  playback.toggle();
  expect(playback.tick(1000)).toBe(false);
  playback.stepForward();
  expect(viewer.frame).toBe(7);
  const at7 = JSON.stringify(viewer.scene());
  playback.stepForward();
  playback.stepBack();
  expect(JSON.stringify(viewer.scene())).toBe(at7);
  playback.seek(650);
  expect(viewer.frame).toBe(650);
  expect(viewer.scene().fighters.map((f) => f.parts.length > 0)).toEqual([true, true]);
  playback.seek(7);
  expect(JSON.stringify(viewer.scene())).toBe(at7);
  playback.seek(699);
  playback.toggle();
  playback.tick(100);
  expect([viewer.frame, playback.playing]).toEqual([700, false]);
  // Play after the last frame starts over.
  playback.toggle();
  expect(viewer.frame).toBe(0);
  expect(clock(3725)).toBe("1:02");
});

test("a replay from a version the client doesn't hold names that version; a kept one plays", async () => {
  const joined = joinedReplay(manifest, parts);
  if (typeof joined === "string") throw new Error(joined);
  const other = joined.map((line) => (line === "version development" ? "version 0123456789ab" : line));
  const missing = await openForWatching(other, simulations());
  expect("problem" in missing && missing.problem).toBe(
    "This replay was recorded on Smashcraft test (version 0123456789ab). This client can play replays from version development; open it in a client that has played 0123456789ab.",
  );
  const kept: Simulation = { ...own, sourceVersion: () => "0123456789ab" };
  const opened = await openForWatching(other, simulations({ "0123456789ab": kept }));
  expect("viewer" in opened).toBe(true);
  expect(await openForWatching(["hello"], simulations())).toEqual({ problem: `This file isn't a Smashcraft replay (not a repro: its first line isn't "wisp-repro 1").` });
});
