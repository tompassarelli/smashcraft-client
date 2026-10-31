import type { Api } from "./api";
import type { ReplayScene, Simulation, Simulations, Watch } from "./replays";

type MapWatchApi = Pick<Api, "keepSim" | "keptSim" | "keptSims" | "mapVersions" | "mapReplayOpen" | "mapReplayStep" | "mapReplayClose">;

const OWN_SIMULATION = "./sim.js";

function asSimulation(module: unknown): Simulation {
  const sim = module as Partial<Simulation>;
  if (typeof sim.openReplay !== "function" || typeof sim.sourceVersion !== "function" || sim.VIEWER_API !== 1) throw new Error("this version can't show replays");
  return sim as Simulation;
}

/** This client's own simulation, kept so a later client can still play its replays, and every one it kept. */
export function simulations(api: MapWatchApi, pageUrl: string): Simulations {
  const ownUrl = new URL(OWN_SIMULATION, pageUrl).href;
  let own: Promise<Simulation> | undefined;
  const loadOwn = async () => {
    const sim = asSimulation(await import(ownUrl));
    const code = await fetch(ownUrl).then((response) => response.text());
    await api.keepSim(sim.sourceVersion(), code).catch(() => undefined);
    return sim;
  };
  return {
    own: () => (own ??= loadOwn()),
    kept: async (version) => {
      const code = await api.keptSim(version);
      if (code === null || code === undefined) return undefined;
      const url = URL.createObjectURL(new Blob([code], { type: "text/javascript" }));
      try {
        return asSimulation(await import(url));
      } finally {
        URL.revokeObjectURL(url);
      }
    },
    map: (version, lines) => mapWatch(api, pageUrl, version, lines),
    held: async () => {
      const versions = new Set([(await (own ??= loadOwn())).sourceVersion(), ...(await api.keptSims()), ...(await api.mapVersions().catch(() => []))]);
      return [...versions];
    },
  };
}

let viewerLua: Promise<string> | undefined;

type Stepped = { frame: number; ended: boolean; scene: ReplayScene } | { problem: string };

/** The replay played in its version's own map, in the app (smashcraft-client:src-tauri/src/mapsim.rs). */
export async function mapWatch(api: MapWatchApi, pageUrl: string, version: string, lines: readonly string[]): Promise<Watch | string | undefined> {
  const viewer = await (viewerLua ??= fetch(new URL("./viewer.lua", pageUrl).href).then((response) => response.text()));
  let answer: string;
  try {
    answer = await api.mapReplayOpen(version, `${lines.join("\n")}\n`, viewer);
  } catch (error) {
    if (String(error).startsWith("no map of version")) return undefined;
    return String(error);
  }
  const { id, opened } = JSON.parse(answer) as { id: number; opened: { first: number; last: number; frame: number } | { problem: string } };
  if ("problem" in opened) {
    api.mapReplayClose(id);
    return opened.problem;
  }
  const step = async (seek: boolean, frames: number) => {
    const stepped = JSON.parse(await api.mapReplayStep(id, seek, frames)) as Stepped;
    if ("problem" in stepped) throw new Error(stepped.problem);
    return stepped;
  };
  let shown = await step(true, opened.first);
  return {
    first: opened.first,
    last: opened.last,
    get frame() {
      return shown.frame;
    },
    get scene() {
      return shown.scene;
    },
    async advance(frames) {
      shown = await step(false, frames);
      return !shown.ended;
    },
    async seek(frame) {
      shown = await step(true, frame);
    },
    close: () => api.mapReplayClose(id),
  };
}
