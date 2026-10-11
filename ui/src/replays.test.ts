// #141: replays listed beside their records, joined from the map's parts,
// copied as one file and watched with pause, frame step and seek; a replay
// from a version the client doesn't hold names that version.
// #159: Warcraft's own replay of a game is listed under the matches played in it.
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import * as ownSimulation from "../kit/sim.js";
import { Playback, clock } from "./playback";
import { type ReplayFile, type Simulation, type Simulations, bundleWatch, type WarcraftGame, joinedReplay, keptName, openForWatching, replayEntries, warcraftGameOf, warcraftName } from "./replays";

/** Lines as Warcraft writes a Preload file. */
const preload = (lines: readonly string[]) => `function PreloadFiles takes nothing returns nothing\n${lines.map((line) => `\tcall Preload( "${line}" )\n`).join("")}endfunction\n`;

const recorded = JSON.parse(readFileSync(join(import.meta.dir, "../kit/fixtures/tape-replay.json"), "utf8")) as { serial: number; manifest: string[]; parts: string[][] };
const TAPE_REPLAY_SERIAL = recorded.serial;
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
  expect(entries.map((e) => [e.build, e.version, e.serial, e.frames, e.parts])).toEqual([["test", own.sourceVersion(), TAPE_REPLAY_SERIAL, 700, parts.length]]);
  expect(entries[0]?.record?.serial).toBe(TAPE_REPLAY_SERIAL);
  expect(refused.map((r) => r.file)).toEqual(["notes.txt"]);
});

test("a replay joined from its parts and copied as one file plays with pause, frame step and seek", async () => {
  expect(joinedReplay(manifest, parts.slice(1))).toBe(`the replay has ${parts.length} parts; ${parts.length - 1} were found`);
  const joined = joinedReplay(manifest, parts);
  if (typeof joined === "string") throw new Error(joined);
  expect(keptName(joined)).toBe(`smashcraft-test-replay-${TAPE_REPLAY_SERIAL}-${own.sourceVersion()}.txt`);
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
  const other = joined.map((line) => (line === `version ${own.sourceVersion()}` ? "version 0123456789ab" : line));
  const missing = await openForWatching(other, simulations());
  expect("problem" in missing && missing.problem).toBe(
    `This replay was recorded on Smashcraft test (version 0123456789ab). This client can play replays from version ${own.sourceVersion()}. To watch it, put that version's map in your Warcraft III Maps folder.`,
  );
  const other_: Simulation = { ...own, sourceVersion: () => "0123456789ab" };
  expect("watch" in (await openForWatching(other, simulations({ "0123456789ab": other_ })))).toBe(true);
  const fromMap = await openForWatching(other, simulations({}, { "0123456789ab": other_ }));
  if ("problem" in fromMap) throw new Error(fromMap.problem);
  expect(await fromMap.watch.advance(30)).toBe(true);
  expect(fromMap.watch.frame).toBe(30);
  expect(await openForWatching(["hello"], simulations())).toEqual({ problem: `This file isn't a Smashcraft replay (not a repro: its first line isn't "wisp-repro 1").` });
});

/** mulberry32: the same cases on every run. */
const random = (seed: number) => () => {
  seed = (seed + 0x6d2b79f5) | 0;
  let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
  t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
};

test("a kept Warcraft game is listed under a tape replay exactly when, in the same folder, it holds that match's record (seed 8, 1000 cases)", () => {
  const next = random(8);
  const pick = <T>(xs: readonly T[]) => xs[Math.floor(next() * xs.length)]!;
  for (let i = 0; i < 1000; i++) {
    const games: WarcraftGame[] = Array.from({ length: Math.floor(next() * 3) }, (_, g) => ({
      file: `warcraft-${g}.w3g`, hash: `${g}`, started: 0, ended: 1, folder: pick(["a", "b"]), replays: "r",
      records: [1, 2, 3, 4].filter(() => next() < 0.4).map((n) => `smashcraft-match-${n}.txt`),
    }));
    const folder = pick(["a", "b"]);
    const serial = pick([1, 2, 3, 4]);
    const parts = pick([undefined, 1, 4]);
    const entry = { key: `${folder}/x`, folder, name: "x", build: "b", version: "v", serial, frames: 1, modified: 1, ...(parts === undefined ? {} : { parts }) };
    const holders = games.filter((game) => game.folder === folder && game.records.includes(`smashcraft-match-${serial}.txt`));
    expect(warcraftGameOf(entry, games)).toBe(parts === undefined ? undefined : holders[0]);
  }
});

test("a kept game's Replays-menu name reads back as the local minute it ended (seed 9, 1000 cases)", () => {
  const next = random(9);
  for (let i = 0; i < 1000; i++) {
    const ended = Math.floor(next() * 4102444800000);
    const name = /^Smashcraft (\d{4})-(\d{2})-(\d{2}) (\d{2})\.(\d{2})$/.exec(warcraftName(ended));
    expect(name).not.toBeNull();
    const [, y, mo, d, h, mi] = name!.map(Number);
    const at = new Date(ended);
    expect([y, mo, d, h, mi]).toEqual([at.getFullYear(), at.getMonth() + 1, at.getDate(), at.getHours(), at.getMinutes()]);
  }
});
