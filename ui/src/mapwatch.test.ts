// The app's answers below are mapsim.rs's own: `{"first":0,"last":700,"frame":0}` from smashcraft_open and
// `{frame, ended, scene}` from smashcraft_advance (smashcraft-client:src-tauri/src/mapsim.rs tests).
import { expect, test } from "bun:test";
import { pathToFileURL } from "node:url";
import { join } from "node:path";
import type { Api } from "./api";
import { mapWatch, simulations } from "./mapwatch";
import type { ReplayScene, Watch } from "./replays";

type FakeApi = Parameters<typeof mapWatch>[0];

const KIT_PAGE = pathToFileURL(join(import.meta.dir, "../kit/index.html")).href;
const scene = (frame: number) => ({ frame }) as unknown as ReplayScene;
const refused = (message: string) => async () => Promise.reject(message);

/** The map simulation's side of the app: answers in order, and every call it took. */
function fakeApi(open: Api["mapReplayOpen"], steps: Awaited<ReturnType<Api["mapReplayStep"]>>[] = []) {
  const calls: unknown[][] = [];
  const api: FakeApi = {
    keepSim: async (version) => void calls.push(["keepSim", version]),
    keptSim: async () => null,
    keptSims: async () => [],
    mapVersions: async () => [],
    mapReplayOpen: open,
    mapReplayStep: async (id, seek, frames) => {
      calls.push(["step", id, seek, frames]);
      return steps.shift()!;
    },
    mapReplayClose: (id) => void calls.push(["close", id]),
  };
  return { api, calls };
}

test("a map replay opens at its first frame, advances and seeks through the app's answers, and says when it ended", async () => {
  const shown = (frame: number, ended = false) => ({ frame, ended, scene: scene(frame) });
  let sent = "";
  const { api, calls } = fakeApi(async (version, replay) => {
    sent = `${version}:${replay}`;
    return { id: 7, opened: { first: 2, last: 700, frame: 0 } };
  }, [shown(2), shown(452), shown(100), shown(700, true)]);
  const watch = (await mapWatch(api, KIT_PAGE, "0123", ["a", "b"])) as Watch;
  expect(sent).toBe("0123:a\nb\n");
  expect([watch.first, watch.last, watch.frame, watch.scene]).toEqual([2, 700, 2, scene(2)]);
  expect(await watch.advance(450)).toBe(true);
  expect(watch.frame).toBe(452);
  await watch.seek(100);
  expect(watch.scene).toEqual(scene(100));
  expect(await watch.advance(1000)).toBe(false);
  watch.close();
  expect(calls).toEqual([["step", 7, true, 2], ["step", 7, false, 450], ["step", 7, true, 100], ["step", 7, false, 1000], ["close", 7]]);
});

test("a version with no map falls through to the caller; any other refusal is shown as the problem", async () => {
  expect(await mapWatch(fakeApi(refused("no map of version ffff was found")).api, KIT_PAGE, "ffff", [])).toBeUndefined();
  expect(await mapWatch(fakeApi(refused("the map broke")).api, KIT_PAGE, "ffff", [])).toBe("the map broke");
});

test("a replay the map refuses to open closes its simulation and shows why", async () => {
  const { api, calls } = fakeApi(async () => ({ id: 3, opened: { problem: "this replay is cut short" } }));
  expect(await mapWatch(api, KIT_PAGE, "0123", [])).toBe("this replay is cut short");
  expect(calls).toEqual([["close", 3]]);
});

test("a step the map refuses rejects with the map's own words", async () => {
  const { api } = fakeApi(async () => ({ id: 3, opened: { first: 0, last: 4, frame: 0 } }), [{ problem: "desynced" }]);
  await expect(mapWatch(api, KIT_PAGE, "0123", [])).rejects.toThrow("desynced");
});

test("the client's own simulation loads once, is kept for later clients even if keeping fails, and heads the held versions without repeats", async () => {
  const { api, calls } = fakeApi(refused("unused"));
  const sims = simulations({ ...api, keepSim: async (version) => (calls.push(["keepSim", version]), Promise.reject("disk full")) }, KIT_PAGE);
  const own = await sims.own();
  expect(await sims.own()).toBe(own);
  expect(calls).toEqual([["keepSim", own.sourceVersion()]]);
  const held = simulations({ ...api, keptSims: async () => [own.sourceVersion(), "old"], mapVersions: async () => ["old", "map"] }, KIT_PAGE);
  expect(await held.held()).toEqual([own.sourceVersion(), "old", "map"]);
  const noMaps = simulations({ ...api, mapVersions: refused("no maps folder") }, KIT_PAGE);
  expect(await noMaps.held()).toEqual([own.sourceVersion()]);
});

test("a kept bundle plays only if it is a viewer; one that isn't is refused and a version never kept is undefined", async () => {
  const { api } = fakeApi(refused("unused"));
  const sims = simulations(api, KIT_PAGE);
  expect(await sims.kept("never")).toBeUndefined();
  const notViewer = simulations({ ...api, keptSim: async () => "export const VIEWER_API = 2; export const sourceVersion = () => 'old'; export const openReplay = () => 'no';" }, KIT_PAGE);
  await expect(notViewer.kept("old")).rejects.toThrow("this version can't show replays");
  const viewer = simulations({ ...api, keptSim: async () => "export const VIEWER_API = 1; export const sourceVersion = () => 'old'; export const openReplay = () => 'no';" }, KIT_PAGE);
  expect((await viewer.kept("old"))?.sourceVersion()).toBe("old");
});
