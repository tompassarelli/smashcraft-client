// Replays the map writes (smashcraft:docs/design/client.md, "Full-match
// replays"): listing them beside their match records, joining a manifest's
// parts into one replay, and opening one in the simulation of the version
// that recorded it. Each version's simulation is its own bundle
// (smashcraft:ts/src/game/replay/viewer.ts, VIEWER_API 1): the client ships
// its own and keeps every one it has played.
import { type ReplayHeader, joinReplay, parseReplayHeader, parseReplayPart } from "../../../ts/src/game/replay/replayFormat";
import type { ReplayScene, ReplayViewer } from "../../../ts/src/game/replay/viewer";
import { type MatchRecord, type RecordFile, parseRecord, preloadLines } from "./records";

export type { ReplayScene, ReplayViewer } from "../../../ts/src/game/replay/viewer";

/** A manifest the map wrote, or a joined replay the client keeps, as the app reads it. */
export type ReplayFile = { folder: string; name: string; text: string; modified: number };

export type ReplayEntry = {
  /** Folder and file name. */
  key: string;
  folder: string;
  name: string;
  build: string;
  version: string;
  serial: number;
  frames: number;
  /** Parts beside a manifest; absent for a joined replay. */
  parts?: number;
  modified: number;
  /** The match record the same client wrote of the same match, when it is beside the replay. */
  record?: MatchRecord;
};

export type ReplayRefusal = { file: string; reason: string };

/** A file's lines: a Preload file's stored lines, or a plain file's. */
export function fileLines(text: string): string[] {
  return preloadLines(text) ?? text.replace(/\r\n/g, "\n").replace(/\n$/, "").split("\n");
}

const recordSerial = (name: string) => /^smashcraft-match-(\d+)\.txt$/.exec(name)?.[1];

/** Each replay with its match record, newest first; files that aren't replays are refused with a reason. */
export function replayEntries(files: readonly ReplayFile[], records: readonly RecordFile[]): { entries: ReplayEntry[]; refused: ReplayRefusal[] } {
  const recordsBy = new Map<string, MatchRecord>();
  for (const file of records) {
    const record = parseRecord(file);
    const serial = recordSerial(file.name);
    if ("id" in record && serial !== undefined) recordsBy.set(`${file.folder}\n${record.build}\n${serial}`, record);
  }
  const entries: ReplayEntry[] = [];
  const refused: ReplayRefusal[] = [];
  for (const file of files) {
    const header = parseReplayHeader(fileLines(file.text));
    if (typeof header === "string") {
      refused.push({ file: file.name, reason: header });
      continue;
    }
    const { repro, serial, version, parts } = header;
    const entry: ReplayEntry = { key: `${file.folder}/${file.name}`, folder: file.folder, name: file.name, build: repro.build, version, serial, frames: repro.frame, modified: file.modified };
    if (parts !== undefined) entry.parts = parts;
    const record = parts === undefined ? undefined : recordsBy.get(`${file.folder}\n${repro.build}\n${serial}`);
    if (record !== undefined) entry.record = record;
    entries.push(entry);
  }
  entries.sort((a, b) => b.modified - a.modified || a.key.localeCompare(b.key));
  return { entries, refused };
}

/** The replay a file holds, joined with its parts when it is a manifest; or what is wrong. */
export function joinedReplay(text: string, parts: readonly string[]): string[] | string {
  const lines = fileLines(text);
  const header: ReplayHeader | string = parseReplayHeader(lines);
  if (typeof header === "string") return header;
  if (header.parts === undefined) return lines;
  if (parts.length !== header.parts) return `the replay has ${header.parts} parts; ${parts.length} were found`;
  const bodies: (readonly string[])[] = [];
  for (const [index, part] of parts.entries()) {
    const body = parseReplayPart(fileLines(part), header.serial, index + 1);
    if (typeof body === "string") return body;
    bodies.push(body);
  }
  return joinReplay(header, bodies);
}

/** A file name to keep a joined replay under, from its build and serial. */
export const keptName = (lines: readonly string[]): string => {
  const header = parseReplayHeader(lines);
  if (typeof header === "string") return `smashcraft-replay-${Date.now()}.txt`;
  return `smashcraft-${header.repro.build}-replay-${header.serial}-${header.version}.txt`.replace(/[^A-Za-z0-9._-]/g, "-");
};

/** What a version's simulation bundle offers (VIEWER_API 1). */
export type Simulation = {
  VIEWER_API: number;
  sourceVersion(): string;
  openReplay(lines: readonly string[]): ReplayViewer | string;
};

/**
 * A replay being watched: the frame shown and its scene, a run of frames
 * forward and a seek. Async, as a replay may play in its map's simulation in
 * the app (smashcraft:client/src-tauri/src/mapsim.rs).
 */
export interface Watch {
  readonly first: number;
  readonly last: number;
  readonly frame: number;
  readonly scene: ReplayScene;
  /** Runs up to `frames` frames; false once the replay ended or a frame couldn't run. */
  advance(frames: number): Promise<boolean>;
  /** Shows the state after `frame`, clamped to the replay. */
  seek(frame: number): Promise<void>;
  close(): void;
}

/** A viewer from a simulation bundle, watched. */
export function bundleWatch(viewer: ReplayViewer): Watch {
  return {
    first: viewer.first,
    last: viewer.last,
    get frame() {
      return viewer.frame;
    },
    get scene() {
      return viewer.scene();
    },
    async advance(frames) {
      for (let step = 0; step < frames; step++) if (!viewer.step()) return false;
      return viewer.frame < viewer.last;
    },
    async seek(frame) {
      viewer.seek(frame);
    },
    close() {},
  };
}

/** Where the client finds a version's simulation: its own, one it kept, or the map of that version in a Maps folder. */
export type Simulations = {
  own(): Promise<Simulation>;
  kept(version: string): Promise<Simulation | undefined>;
  /** The replay opened in its version's map; undefined when no map of that version was found, a string when it can't play. */
  map(version: string, lines: readonly string[]): Promise<Watch | string | undefined>;
  /** Versions the client can play: its own, every one kept and every map found. */
  held(): Promise<string[]>;
};

export type Opened = { watch: Watch } | { problem: string };

/** The words naming a version a player can tell apart. */
export const versionName = (version: string, build: string) => `Smashcraft ${build} (version ${version})`;

/** A joined replay opened in the simulation of the version that recorded it, or why it can't be. */
export async function openForWatching(lines: readonly string[], simulations: Simulations): Promise<Opened> {
  const header = parseReplayHeader(lines);
  if (typeof header === "string") return { problem: `This file isn't a Smashcraft replay (${header}).` };
  const own = await simulations.own();
  const simulation = own.sourceVersion() === header.version ? own : await simulations.kept(header.version);
  if (simulation !== undefined) {
    const viewer = simulation.openReplay(lines);
    return typeof viewer === "string" ? { problem: `This replay can't be played: ${viewer}.` } : { watch: bundleWatch(viewer) };
  }
  const watch = await simulations.map(header.version, lines);
  if (typeof watch === "string") return { problem: `This replay can't be played: ${watch}.` };
  if (watch !== undefined) return { watch };
  const held = await simulations.held();
  return {
    problem: `This replay was recorded on ${versionName(header.version, header.repro.build)}. This client can play replays from version ${held.join(", ") || "none"}. To watch it, put that version's map in your Warcraft III Maps folder.`,
  };
}

/** A Warcraft game the client kept (#159): Warcraft's own replay of a whole lobby session, every rematch included. */
export type WarcraftGame = { file: string; hash: string; started: number; ended: number; folder: string; replays: string; records: string[] };

/** The kept Warcraft game a replay's match was played in: the one whose records include the match's record. */
export const warcraftGameOf = (entry: ReplayEntry, games: readonly WarcraftGame[]): WarcraftGame | undefined =>
  entry.parts === undefined ? undefined : games.find((game) => game.folder === entry.folder && game.records.includes(`smashcraft-match-${entry.serial}.txt`));

/** The name a kept game gets in Warcraft III's Replays menu, from when it ended (local time). */
export function warcraftName(ended: number): string {
  const at = new Date(ended);
  const two = (n: number) => String(n).padStart(2, "0");
  return `Smashcraft ${at.getFullYear()}-${two(at.getMonth() + 1)}-${two(at.getDate())} ${two(at.getHours())}.${two(at.getMinutes())}`;
}
