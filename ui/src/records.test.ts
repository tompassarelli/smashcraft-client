import { expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { duration, emptyStore, ingest, parseRecord, type RecordFile } from "./records";
import { computeStats, perMatch, percent } from "./stats";

const FIXTURES = join(import.meta.dir, "../test/fixtures/records");
const RECORD_NAME = /^smashcraft-match-\d+\.txt$/;

/** The fixture folders' record files as the app reads them (smashcraft-client:src-tauri/src/records.rs), written a minute apart in order. */
function fixtureFiles(): RecordFile[] {
  const files: RecordFile[] = [];
  for (const folder of ["a", "b"]) {
    for (const name of readdirSync(join(FIXTURES, folder)).filter((n) => RECORD_NAME.test(n)).sort()) {
      files.push({ folder, name, text: readFileSync(join(FIXTURES, folder, name), "utf8"), modified: (files.length + 1) * 60_000 });
    }
  }
  return files;
}

test("ingesting the fixture folders stores each match once, skips the duplicate and refuses the cut-short record", () => {
  const files = fixtureFiles();
  expect(files.map((f) => `${f.folder}/${f.name}`)).toEqual([
    "a/smashcraft-match-1.txt", "a/smashcraft-match-2.txt", "a/smashcraft-match-3.txt", "a/smashcraft-match-4.txt",
    "b/smashcraft-match-1.txt", "b/smashcraft-match-7.txt",
  ]);
  const first = ingest(emptyStore(), files);
  expect(first.added.map((r) => r.id)).toEqual(["0.0.60|1|Tom#1234", "0.0.60|2|Tom#1234", "0.0.58|4|Tom#1234", "0.0.60|7|Rival#5678"]);
  expect(first.duplicates).toBe(1);
  expect(first.refused).toEqual([{ file: "a/smashcraft-match-3.txt", reason: "cut short" }]);
  // The history lists the newest first.
  expect(first.store.records.map((r) => r.serial)).toEqual([7, 4, 2, 1]);

  const match = first.store.records.find((r) => r.serial === 1)!;
  expect(match).toMatchObject({ build: "0.0.60", local: "P1", mode: "versus", winner: "P1", stage: "Frozen Throne", frames: 4521, stocks: 3, minutes: 7 });
  expect(match.fighters).toEqual([
    { slot: "P1", computer: false, player: "Tom#1234", character: 4, hero: "Mountain King", stocks: 2, damage: 37, kos: 3, falls: 1, left: false,
      combat: { dealt: 141, openings: 9, techs: 2, missedTechs: 1, ledgeGrabs: 4 } },
    { slot: "P2", computer: true, character: 6, hero: "Lich", stocks: 0, damage: 112, kos: 1, falls: 3, left: false,
      combat: { dealt: 36, openings: 3, techs: 0, missedTechs: 0, ledgeGrabs: 1 } },
  ]);
  expect(first.store.records.find((r) => r.serial === 4)!.fighters[0]!.combat).toBeUndefined();
  expect(duration(match.frames)).toBe("1:15");

  // Ingesting the same folders again adds nothing.
  const again = ingest(first.store, files);
  expect(again.added).toEqual([]);
  expect(again.duplicates).toBe(5);
  expect(again.store.records.length).toBe(4);
});

test("a record file that isn't whole, or whose lines don't parse, is refused", () => {
  const [file] = fixtureFiles();
  const whole = file!.text;
  expect(parseRecord({ ...file!, text: whole.slice(0, whole.indexOf("endfunction")) })).toEqual({ file: "a/smashcraft-match-1.txt", reason: "not a whole record file" });
  expect(parseRecord({ ...file!, text: whole.replace("kos=3", "kos=x") })).toEqual({ file: "a/smashcraft-match-1.txt", reason: "kos isn't a count" });
});

test("the stats page's numbers from the fixture records: win rate per fighter and matchup, KOs and falls per match, average damage", () => {
  const { store } = ingest(emptyStore(), fixtureFiles());
  const stats = computeStats(store.records);
  expect(stats.matches).toBe(4);
  const fighter = (hero: string) => stats.fighters.find((f) => f.hero === hero)!;
  expect(stats.fighters[0]!.hero).toBe("Mountain King");
  expect(fighter("Mountain King")).toEqual({ hero: "Mountain King", matches: 2, wins: 2, kos: 6, falls: 3, dealt: 141, combatMatches: 1 });
  expect(fighter("Illidan")).toEqual({ hero: "Illidan", matches: 1, wins: 0, kos: 2, falls: 3, dealt: 99, combatMatches: 1 });
  expect(fighter("Lich")).toEqual({ hero: "Lich", matches: 1, wins: 1, kos: 3, falls: 1, dealt: 200, combatMatches: 1 });
  expect(stats.fighters.length).toBe(3);

  const king = fighter("Mountain King");
  expect(percent(king.wins, king.matches)).toBe("100%");
  expect(perMatch(king.kos, king.matches)).toBe("3.0");
  expect(perMatch(king.falls, king.matches)).toBe("1.5");
  expect(perMatch(king.dealt, king.combatMatches)).toBe("141.0");
  expect(percent(0, 0)).toBe("–");

  const matchups = stats.matchups.map((m) => `${m.hero} vs ${m.opponent}: ${m.wins}/${m.matches}`).sort();
  expect(matchups).toEqual(["Illidan vs Lich: 0/1", "Lich vs Mountain King: 1/1", "Mountain King vs Lich: 1/1", "Mountain King vs Rifleman: 1/1"]);
});
