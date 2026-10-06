// Match records the map writes (smashcraft:docs/design/client.md, "Match
// records") and the client's history store they are ingested into.

/** A record file as the app reads it from a CustomMapData folder. */
export type RecordFile = { folder: string; name: string; text: string; modified: number };

export type Combat = { dealt: number; openings: number; techs: number; missedTechs: number; ledgeGrabs: number };

export type FighterRecord = {
  slot: string;
  computer: boolean;
  /** The account name; absent for a computer. */
  player?: string;
  character: number;
  hero: string;
  stocks: number;
  damage: number;
  kos: number;
  falls: number;
  left: boolean;
  /** Absent in records written before combat stats existed. */
  combat?: Combat;
};

export type MatchRecord = {
  /** Build, serial and the writer's account: the same match from the same player is one record. */
  id: string;
  build: string;
  serial: number;
  /** The writer's slot, or "none" for an observer. */
  local: string;
  mode: string;
  stocks: number;
  minutes: number;
  frames: number;
  /** The winner's slot, or "none". */
  winner: string;
  timedOut: boolean;
  interrupted: boolean;
  stage: string;
  fighters: FighterRecord[];
  /** When the record was written, in milliseconds since 1970. */
  playedAt: number;
};

export type HistoryStore = { version: 1; records: MatchRecord[] };

export const emptyStore = (): HistoryStore => ({ version: 1, records: [] });

/** A stored value, or an empty store when it isn't one. */
export function asStore(value: unknown): HistoryStore {
  const store = value as Partial<HistoryStore> | null;
  return store?.version === 1 && Array.isArray(store.records) ? (store as HistoryStore) : emptyStore();
}

const PRELOAD_HEADER = /^function PreloadFiles takes nothing returns nothing\r?\n/;
const PRELOAD_LINE = /^\s*call Preload\( "(.*)" \)[\t ]*\r?$/gm;

/** The lines a whole Preload file holds, or undefined when it isn't one or isn't whole. */
export function preloadLines(text: string): string[] | undefined {
  if (!PRELOAD_HEADER.test(text) || !text.trimEnd().endsWith("endfunction")) return undefined;
  return [...text.matchAll(PRELOAD_LINE)].map((match) => match[1] ?? "");
}

type Fields = Map<string, string>;

/** A line's word and its key=value fields; `name=` runs to the end of the line. */
function fieldsOf(line: string): { word: string; fields: Fields } {
  const named = line.indexOf(" name=");
  const head = named < 0 ? line : line.slice(0, named);
  const [word = "", ...pairs] = head.split(" ");
  const fields: Fields = new Map();
  for (const pair of pairs) {
    const at = pair.indexOf("=");
    if (at > 0) fields.set(pair.slice(0, at), pair.slice(at + 1));
  }
  if (named >= 0) fields.set("name", line.slice(named + " name=".length));
  return { word, fields };
}

class Malformed extends Error {}

function text(fields: Fields, key: string): string {
  const value = fields.get(key);
  if (value === undefined || value === "") throw new Malformed(`no ${key}`);
  return value;
}

function count(fields: Fields, key: string): number {
  const value = Number(text(fields, key));
  if (!Number.isInteger(value) || value < 0) throw new Malformed(`${key} isn't a count`);
  return value;
}

const flag = (fields: Fields, key: string) => text(fields, key) === "1";

/** Why a record file was not ingested. */
export type Refusal = { file: string; reason: string };

/** The record a file holds, or why it can't be read: a record whose end line doesn't count its lines was cut short. */
export function parseRecord(file: RecordFile): MatchRecord | Refusal {
  const where = `${file.folder}/${file.name}`;
  const lines = preloadLines(file.text);
  if (lines === undefined || lines.length === 0) return { file: where, reason: "not a whole record file" };
  const end = fieldsOf(lines[lines.length - 1] ?? "");
  if (end.word !== "end" || Number(end.fields.get("lines")) !== lines.length - 1) return { file: where, reason: "cut short" };
  try {
    const parsed = lines.slice(0, -1).map(fieldsOf);
    const header = parsed[0];
    if (header?.word !== "smashcraft-match") throw new Malformed("not a match record");
    const only = (word: string) => {
      const line = parsed.find((p) => p.word === word);
      if (line === undefined) throw new Malformed(`no ${word} line`);
      return line.fields;
    };
    const rules = only("rules");
    const result = only("result");
    const fighters: FighterRecord[] = [];
    for (const { word, fields } of parsed) {
      if (word === "fighter") {
        const computer = text(fields, "kind") === "computer";
        fighters.push({
          slot: text(fields, "slot"),
          computer,
          ...(computer ? {} : { player: text(fields, "player") }),
          character: count(fields, "character"),
          hero: text(fields, "name"),
          stocks: count(fields, "stocks"),
          damage: count(fields, "damage"),
          kos: count(fields, "kos"),
          falls: count(fields, "falls"),
          left: flag(fields, "left"),
        });
      } else if (word === "combat") {
        const fighter = fighters.find((f) => f.slot === fields.get("slot"));
        if (fighter === undefined) throw new Malformed("combat line without its fighter");
        fighter.combat = {
          dealt: count(fields, "dealt"),
          openings: count(fields, "openings"),
          techs: count(fields, "techs"),
          missedTechs: count(fields, "missed-techs"),
          ledgeGrabs: count(fields, "ledge-grabs"),
        };
      }
    }
    if (fighters.length === 0) throw new Malformed("no fighters");
    const build = text(header.fields, "build");
    const serial = count(header.fields, "serial");
    const local = text(header.fields, "local");
    const writer = fighters.find((f) => f.slot === local)?.player ?? "observer";
    return {
      id: `${build}|${serial}|${writer}`,
      build,
      serial,
      local,
      mode: text(header.fields, "mode"),
      stocks: count(rules, "stocks"),
      minutes: count(rules, "minutes"),
      frames: count(result, "frames"),
      winner: text(result, "winner"),
      timedOut: flag(result, "timed-out"),
      interrupted: flag(result, "interrupted"),
      stage: text(only("stage"), "name"),
      fighters,
      playedAt: file.modified,
    };
  } catch (error) {
    if (error instanceof Malformed) return { file: where, reason: error.message };
    throw error;
  }
}

export type IngestResult = { store: HistoryStore; added: MatchRecord[]; duplicates: number; refused: Refusal[] };

/** Adds the files' new records to the store, newest first; records it already holds are skipped. */
export function ingest(store: HistoryStore, files: readonly RecordFile[]): IngestResult {
  const known = new Set(store.records.map((record) => record.id));
  const added: MatchRecord[] = [];
  const refused: Refusal[] = [];
  let duplicates = 0;
  for (const file of files) {
    const parsed = parseRecord(file);
    if ("reason" in parsed) refused.push(parsed);
    else if (known.has(parsed.id)) duplicates++;
    else {
      known.add(parsed.id);
      added.push(parsed);
    }
  }
  const records = [...store.records, ...added].sort((a, b) => b.playedAt - a.playedAt || b.serial - a.serial);
  return { store: { version: 1, records }, added, duplicates, refused };
}

/** The fighter whose player wrote the record; undefined for an observer's. */
export const writerOf = (record: MatchRecord): FighterRecord | undefined => record.fighters.find((f) => f.slot === record.local);

/** A match's length as m:ss. */
export function duration(frames: number): string {
  const seconds = Math.floor(frames / 60);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}
