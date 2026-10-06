// Replays the map writes (smashcraft:docs/design/client.md, "Full-match
// replays"): listing them beside their match records, joining a manifest's
// parts into one replay, and opening one in the simulation of the version
// that recorded it. Each version's simulation is its own bundle
// (smashcraft:ts/src/game/replay/viewer.ts, VIEWER_API 1): the client ships
// its own and keeps every one it has played.
import { type ReplayHeader, joinReplay, parseReplayHeader, parseReplayPart } from "../../../ts/src/game/replay/replayFormat";
import type { ReplayViewer } from "../../../ts/src/game/replay/viewer";
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

/** Where the client finds a version's simulation: its own, or one it kept. */
export type Simulations = {
  own(): Promise<Simulation>;
  kept(version: string): Promise<Simulation | undefined>;
  /** Versions the client can play: its own and every one kept. */
  held(): Promise<string[]>;
};

export type Opened = { viewer: ReplayViewer } | { problem: string };

/** The words naming a version a player can tell apart. */
export const versionName = (version: string, build: string) => `Smashcraft ${build} (version ${version})`;

/** A joined replay opened in the simulation of the version that recorded it, or why it can't be. */
export async function openForWatching(lines: readonly string[], simulations: Simulations): Promise<Opened> {
  const header = parseReplayHeader(lines);
  if (typeof header === "string") return { problem: `This file isn't a Smashcraft replay (${header}).` };
  const own = await simulations.own();
  const simulation = own.sourceVersion() === header.version ? own : await simulations.kept(header.version);
  if (simulation === undefined) {
    const held = await simulations.held();
    return {
      problem: `This replay was recorded on ${versionName(header.version, header.repro.build)}. This client can play replays from version ${held.join(", ") || "none"}; open it in a client that has played ${header.version}.`,
    };
  }
  const viewer = simulation.openReplay(lines);
  return typeof viewer === "string" ? { problem: `This replay can't be played: ${viewer}.` } : { viewer };
}
