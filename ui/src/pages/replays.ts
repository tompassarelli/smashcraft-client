import { h } from "../dom";
import { drawScene } from "../draw";
import { Playback, clock } from "../playback";
import { duration } from "../records";
import {
  type ReplayEntry, type ReplayScene, type Simulation, type Simulations, type WarcraftGame, type Watch, joinedReplay, keptName, openForWatching, replayEntries, versionName,
  warcraftGameOf, warcraftName,
} from "../replays";
import { api } from "../tauri";

const OWN_SIMULATION = "./sim.js";

function asSimulation(module: unknown): Simulation {
  const sim = module as Partial<Simulation>;
  if (typeof sim.openReplay !== "function" || typeof sim.sourceVersion !== "function" || sim.VIEWER_API !== 1) throw new Error("this version can't show replays");
  return sim as Simulation;
}

/** This client's own simulation, kept so a later client can still play its replays, and every one it kept. */
function simulations(): Simulations {
  const ownUrl = new URL(OWN_SIMULATION, location.href).href;
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
    map: (version, lines) => mapWatch(version, lines),
    held: async () => {
      const versions = new Set([(await (own ??= loadOwn())).sourceVersion(), ...(await api.keptSims()), ...(await api.mapVersions().catch(() => []))]);
      return [...versions];
    },
  };
}

let viewerLua: Promise<string> | undefined;

type Stepped = { frame: number; ended: boolean; scene: ReplayScene } | { problem: string };

/** The replay played in its version's own map, in the app (smashcraft:client/src-tauri/src/mapsim.rs). */
async function mapWatch(version: string, lines: readonly string[]): Promise<Watch | string | undefined> {
  const viewer = await (viewerLua ??= fetch(new URL("./viewer.lua", location.href).href).then((response) => response.text()));
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

const SLOTS = ["P1", "P2", "P3", "P4"];

function title(entry: ReplayEntry): string {
  const record = entry.record;
  if (record === undefined) return `Replay ${entry.serial}`;
  const winner = record.fighters.find((f) => f.slot === record.winner);
  return `${record.fighters.map((f) => f.hero).join(" vs ")} · ${record.interrupted ? "ended early" : winner ? `${winner.hero} won` : "draw"}`;
}

/** The viewer: a canvas, play and pause, a frame back and forward, and a seek bar. */
function viewerCard() {
  const canvas = h("canvas.replay-canvas", { width: 960, height: 560 });
  const playButton = h("button.primary", { type: "button" }, "Play");
  const back = h("button", { type: "button", title: "A frame back (←)" }, "◀ Frame");
  const forward = h("button", { type: "button", title: "A frame forward (→)" }, "Frame ▶");
  const seek = h("input.seek", { type: "range", min: "0", max: "0", step: "1", value: "0" });
  const label = h("span.muted.frame-label");
  const heading = h("h2");
  const problem = h("p.problem", { hidden: true });
  const el = h("section.card", { hidden: true }, heading, problem, canvas, h("div.replay-controls", {}, back, playButton, forward, seek, label));
  let playback: Playback | undefined;
  let names: string[] = [];
  let frameRequest = 0;
  let last = 0;
  const render = () => {
    if (playback === undefined) return;
    const { watch } = playback;
    drawScene(canvas, watch.scene, names);
    seek.value = String(watch.frame);
    label.textContent = `${clock(watch.frame - watch.first)} / ${clock(watch.last - watch.first)} · frame ${watch.frame}`;
    playButton.textContent = playback.playing ? "Pause" : "Play";
  };
  const animate = (now: number) => {
    frameRequest = 0;
    if (playback === undefined || !playback.playing) return;
    const elapsed = last === 0 ? 0 : now - last;
    last = now;
    void playback.tick(elapsed).then((changed) => changed && render());
    render();
    frameRequest = requestAnimationFrame(animate);
  };
  const loop = () => {
    last = 0;
    if (frameRequest === 0 && playback?.playing) frameRequest = requestAnimationFrame(animate);
    render();
  };
  const act = (action: (shown: Playback) => Promise<void>) => {
    if (playback !== undefined) void action(playback).then(loop);
  };
  playButton.addEventListener("click", () => act((shown) => shown.toggle()));
  back.addEventListener("click", () => act((shown) => shown.stepBack()));
  forward.addEventListener("click", () => act((shown) => shown.stepForward()));
  seek.addEventListener("input", () => act((shown) => shown.seek(Number(seek.value))));
  const keys = (event: KeyboardEvent) => {
    if (playback === undefined || el.hidden || event.target instanceof HTMLInputElement && event.target.type === "text") return;
    if (event.key === " ") act((shown) => shown.toggle());
    else if (event.key === "ArrowLeft") act((shown) => shown.stepBack());
    else if (event.key === "ArrowRight") act((shown) => shown.stepForward());
    else return;
    event.preventDefault();
  };
  window.addEventListener("keydown", keys);
  return {
    el,
    show(text: string, opened: Playback | undefined, fighterNames: string[], message?: string) {
      playback?.pause();
      playback?.watch.close();
      playback = opened;
      names = fighterNames;
      heading.textContent = text;
      el.hidden = false;
      problem.hidden = message === undefined;
      problem.textContent = message ?? "";
      for (const part of [canvas, back, playButton, forward, seek, label]) part.hidden = opened === undefined;
      if (opened !== undefined) {
        seek.min = String(opened.watch.first);
        seek.max = String(opened.watch.last);
      }
      loop();
      el.scrollIntoView({ behavior: "smooth", block: "start" });
    },
    stop() {
      playback?.pause();
      playback?.watch.close();
      cancelAnimationFrame(frameRequest);
      window.removeEventListener("keydown", keys);
    },
  };
}

export function replaysPage(root: HTMLElement): () => void {
  const sims = simulations();
  const list = h("ol.matches");
  const status = h("p.note");
  const refreshButton = h("button.primary", { type: "button" }, "Look for new replays");
  const file = h("input", { type: "file", accept: ".txt", hidden: true });
  const openFile = h("button", { type: "button" }, "Open a replay file…");
  const viewer = viewerCard();
  let live = true;

  const watch = async (heading: string, lines: string[] | string, names: string[]) => {
    if (typeof lines === "string") {
      viewer.show(heading, undefined, names, `This replay can't be read: ${lines}.`);
      return;
    }
    const opened = await openForWatching(lines, sims);
    if (!live) return;
    if ("problem" in opened) viewer.show(heading, undefined, names, opened.problem);
    else viewer.show(heading, new Playback(opened.watch), names);
  };

  const replayLines = async (entry: ReplayEntry, text: string) =>
    joinedReplay(text, entry.parts === undefined ? [] : await api.readReplayParts(entry.folder, entry.serial, entry.parts));

  /** Puts Warcraft's replay of the whole game into Warcraft III's Replays menu. */
  const inWarcraft = (game: WarcraftGame, saved: HTMLElement) => {
    const button = h("button", { type: "button", title: "Warcraft III's own replay of the game this match was in, every rematch included" }, "Watch in Warcraft");
    button.addEventListener("click", async () => {
      const name = warcraftName(game.ended);
      try {
        await api.watchInWarcraft(game.file, name);
        saved.textContent = `In Warcraft III, open Replays and choose “${name}”.`;
      } catch (error) {
        saved.textContent = `Couldn't put this replay in Warcraft III's Replays: ${String(error)}`;
      }
    });
    return button;
  };

  const row = (entry: ReplayEntry, text: string, game: WarcraftGame | undefined) => {
    const names = SLOTS.map((slot) => entry.record?.fighters.find((f) => f.slot === slot)?.hero ?? slot);
    const watchButton = h("button.primary", { type: "button" }, "Watch");
    watchButton.addEventListener("click", async () => void watch(title(entry), await replayLines(entry, text), names));
    const share = h("button", { type: "button", title: "Keep the whole replay as one file you can copy to another computer" }, "Save a copy to share");
    const saved = h("span.muted.saved");
    share.addEventListener("click", async () => {
      const lines = await replayLines(entry, text);
      if (typeof lines === "string") saved.textContent = `This replay can't be read: ${lines}.`;
      else saved.textContent = `Saved as ${await api.keepReplay(keptName(lines), `${lines.join("\n")}\n`)}`;
    });
    const record = entry.record;
    const when = entry.modified > 0 ? new Date(entry.modified).toLocaleString() : "";
    return h(
      "li.match",
      {},
      h(
        "div.match-head",
        {},
        h("span.result", {}, title(entry)),
        h("span.muted", {}, [record?.stage, duration(entry.frames)].filter(Boolean).join(" · ")),
        h("span.muted", {}, versionName(entry.version, entry.build)),
        h("span.muted.when", {}, when),
      ),
      h("div.replay-actions", {}, watchButton, entry.parts !== undefined ? share : null, game === undefined ? null : inWarcraft(game, saved), saved),
    );
  };

  const refresh = async () => {
    refreshButton.disabled = true;
    try {
      const [files, records, games] = await Promise.all([api.readReplays(), api.readRecords(), api.warcraftGames()]);
      if (!live) return;
      const { entries, refused } = replayEntries(files, records);
      const texts = new Map(files.map((f) => [`${f.folder}/${f.name}`, f.text]));
      list.replaceChildren(...entries.map((entry) => row(entry, texts.get(entry.key) ?? "", warcraftGameOf(entry, games))));
      if (entries.length === 0) list.append(h("li.muted", {}, "No replays yet. Every match you finish is saved as a replay in the folders on the History page."));
      status.textContent = refused.length > 0 ? `${refused.length} couldn't be read (${refused.map((r) => `${r.file}: ${r.reason}`).join("; ")})` : "";
    } catch (error) {
      status.textContent = String(error);
    } finally {
      refreshButton.disabled = false;
    }
  };

  refreshButton.addEventListener("click", () => void refresh());
  openFile.addEventListener("click", () => file.click());
  file.addEventListener("change", async () => {
    const chosen = file.files?.[0];
    file.value = "";
    if (chosen === undefined) return;
    const lines = joinedReplay(await chosen.text(), []);
    if (typeof lines !== "string") await api.keepReplay(keptName(lines), `${lines.join("\n")}\n`).catch(() => undefined);
    await watch(chosen.name, lines, SLOTS);
    void refresh();
  });

  root.append(
    h("section.card", {}, h("div.card-head", {}, h("h2", {}, "Replays"), h("div.replay-actions", {}, openFile, refreshButton)), status, list, file),
    viewer.el,
  );
  void refresh();
  return () => {
    live = false;
    viewer.stop();
  };
}
