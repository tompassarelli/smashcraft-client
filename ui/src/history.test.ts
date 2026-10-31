import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { refreshHistory } from "./history";
import type { HistoryStore, RecordFile } from "./records";

const FIXTURE = join(import.meta.dir, "../test/fixtures/records/a");
const record = (name: string): RecordFile => ({ folder: "a", name, text: readFileSync(join(FIXTURE, name), "utf8"), modified: 60_000 });

/** The app's history file and record folders. */
function fakeApi(files: RecordFile[]) {
  const stored: { history: unknown; saves: number } = { history: null, saves: 0 };
  const api: Parameters<typeof refreshHistory>[0] = {
    history: async () => stored.history,
    readRecords: async () => files,
    saveHistory: async (history: HistoryStore) => {
      stored.history = history;
      stored.saves += 1;
    },
  };
  return { api, stored };
}

test("refreshing history saves new matches once and a second look at the same folders writes nothing", async () => {
  const { api, stored } = fakeApi([record("smashcraft-match-1.txt"), record("smashcraft-match-2.txt")]);
  const first = await refreshHistory(api);
  expect(first.added).toHaveLength(2);
  expect(stored.saves).toBe(1);
  const second = await refreshHistory(api);
  expect(second.added).toEqual([]);
  expect(second.store.records).toHaveLength(2);
  expect(stored.saves).toBe(1);
});
