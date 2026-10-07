// #141: replays listed beside their records, joined from the map's parts,
// copied as one file and watched with pause, frame step and seek; a replay
// from a version the client doesn't hold names that version.
// #159: Warcraft's own replay of a game is listed under the matches played in it.
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { TAPE_REPLAY_SERIAL, recordTapeReplay } from "../../../ts/src/game/replay/tapeReplay";
import * as ownSimulation from "../../../ts/src/game/replay/viewerBundle";
import { Playback, clock } from "./playback";
import { type ReplayFile, type Simulation, type Simulations, bundleWatch, type WarcraftGame, joinedReplay, keptName, openForWatching, replayEntries, warcraftGameOf, warcraftName } from "./replays";

/** Lines as Warcraft writes a Preload file. */
const preload = (lines: readonly string[]) => `function PreloadFiles takes nothing returns nothing\n${lines.map((line) => `\tcall Preload( "${line}" )\n`).join("")}endfunction\n`;

const recorded = recordTapeReplay(700, 401);
const manifest = preload(recorded.manifest);
const parts = recorded.parts.map(preload);
const own: Simulation = ownSimulation;

/** Kept bundles by version, and the maps found in Maps folders as a stand-in for the app's map simulation. */
const simulations = (kept: Record<string, Simulation> = {}, maps: Record<string, Simulation> = {}): Simulations => ({
  own: async () => own,
  kept: async (version) => kept[version],
  map: async (version, lines) => {
    const map = maps[version];
    if (map === undefined) return undefined;
    const viewer = map.openReplay(lines);
    return typeof viewer === "string" ? viewer : bundleWatch(viewer);
  },
  held: async () => [own.sourceVersion(), ...Object.keys(kept), ...Object.keys(maps)],
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
  const playback = new Playback(opened.watch);
  const { watch } = playback;
  expect([watch.first, watch.last, watch.frame]).toEqual([0, 700, 0]);
  await playback.toggle();
  expect(await playback.tick(1000)).toBe(true);
  expect(watch.frame).toBe(6);
  await playback.toggle();
  expect(await playback.tick(1000)).toBe(false);
  await playback.stepForward();
  expect(watch.frame).toBe(7);
  const at7 = JSON.stringify(watch.scene);
  await playback.stepForward();
  await playback.stepBack();
  expect(JSON.stringify(watch.scene)).toBe(at7);
  await playback.seek(650);
  expect(watch.frame).toBe(650);
  expect(watch.scene.fighters.map((f) => f.parts.length > 0)).toEqual([true, true]);
  await playback.seek(7);
  expect(JSON.stringify(watch.scene)).toBe(at7);
  await playback.seek(699);
  await playback.toggle();
  await playback.tick(100);
  expect([watch.frame, playback.playing]).toEqual([700, false]);
  // Play after the last frame starts over.
  await playback.toggle();
  expect(watch.frame).toBe(0);
  expect(clock(3725)).toBe("1:02");
});

test("a replay from a version the client doesn't hold names that version; a kept bundle or a map of that version plays it", async () => {
  const joined = joinedReplay(manifest, parts);
  if (typeof joined === "string") throw new Error(joined);
  const other = joined.map((line) => (line === "version development" ? "version 0123456789ab" : line));
  const missing = await openForWatching(other, simulations());
  expect("problem" in missing && missing.problem).toBe(
    "This replay was recorded on Smashcraft test (version 0123456789ab). This client can play replays from version development. To watch it, put that version's map in your Warcraft III Maps folder.",
  );
  const other_: Simulation = { ...own, sourceVersion: () => "0123456789ab" };
  expect("watch" in (await openForWatching(other, simulations({ "0123456789ab": other_ })))).toBe(true);
  const fromMap = await openForWatching(other, simulations({}, { "0123456789ab": other_ }));
  if ("problem" in fromMap) throw new Error(fromMap.problem);
  expect(await fromMap.watch.advance(30)).toBe(true);
  expect(fromMap.watch.frame).toBe(30);
  expect(await openForWatching(["hello"], simulations())).toEqual({ problem: `This file isn't a Smashcraft replay (not a repro: its first line isn't "wisp-repro 1").` });
});

test("a kept Warcraft game is listed under each match whose record it holds, in the same folder", () => {
  const entry = (folder: string, serial: number, parts?: number) =>
    ({ key: `${folder}/smashcraft-replay-${serial}.txt`, folder, name: `smashcraft-replay-${serial}.txt`, build: "b", version: "v", serial, frames: 1, modified: 1, ...(parts === undefined ? {} : { parts }) }) as const;
  const game: WarcraftGame = { file: "warcraft-20261007-023258-abc.w3g", hash: "abc", started: 0, ended: 1, folder: "a", replays: "a/../BattleNet/1/Replays", records: ["smashcraft-match-2.txt", "smashcraft-match-3.txt"] };
  const games = [game];
  expect([entry("a", 2, 1), entry("a", 3, 4), entry("a", 4, 1), entry("b", 2, 1), entry("a", 2)].map((e) => warcraftGameOf(e, games)?.file)).toEqual([game.file, game.file, undefined, undefined, undefined]);
  expect(warcraftName(new Date(2026, 9, 7, 10, 32, 58).getTime())).toBe("Smashcraft 2026-10-07 10.32");
});
