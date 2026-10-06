import { h } from "../dom";
import { drawScene } from "../draw";
import { Playback, clock } from "../playback";
import { duration } from "../records";
import {
  type ReplayEntry, type Simulation, type Simulations, joinedReplay, keptName, openForWatching, replayEntries, versionName,
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
    held: async () => {
      const versions = new Set([(await (own ??= loadOwn())).sourceVersion(), ...(await api.keptSims())]);
      return [...versions];
    },
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
    const { viewer } = playback;
    drawScene(canvas, viewer.scene(), names);
    seek.value = String(viewer.frame);
    label.textContent = `${clock(viewer.frame - viewer.first)} / ${clock(viewer.last - viewer.first)} · frame ${viewer.frame}`;
    playButton.textContent = playback.playing ? "Pause" : "Play";
  };
  const animate = (now: number) => {
    frameRequest = 0;
    if (playback === undefined || !playback.playing) return;
    const elapsed = last === 0 ? 0 : now - last;
    last = now;
    playback.tick(elapsed);
    render();
    frameRequest = requestAnimationFrame(animate);
  };
  const loop = () => {
    last = 0;
    if (frameRequest === 0 && playback?.playing) frameRequest = requestAnimationFrame(animate);
    render();
  };
  playButton.addEventListener("click", () => {
    playback?.toggle();
    loop();
  });
  back.addEventListener("click", () => {
    playback?.stepBack();
    render();
  });
  forward.addEventListener("click", () => {
    playback?.stepForward();
    render();
  });
  seek.addEventListener("input", () => {
    playback?.seek(Number(seek.value));
    render();
  });
  const keys = (event: KeyboardEvent) => {
    if (playback === undefined || el.hidden || event.target instanceof HTMLInputElement && event.target.type === "text") return;
    if (event.key === " ") playback.toggle();
    else if (event.key === "ArrowLeft") playback.stepBack();
    else if (event.key === "ArrowRight") playback.stepForward();
    else return;
    event.preventDefault();
    loop();
  };
  window.addEventListener("keydown", keys);
  return {
    el,
    show(text: string, opened: Playback | undefined, fighterNames: string[], message?: string) {
      playback?.pause();
      playback = opened;
      names = fighterNames;
      heading.textContent = text;
      el.hidden = false;
      problem.hidden = message === undefined;
      problem.textContent = message ?? "";
      for (const part of [canvas, back, playButton, forward, seek, label]) part.hidden = opened === undefined;
      if (opened !== undefined) {
        seek.min = String(opened.viewer.first);
        seek.max = String(opened.viewer.last);
      }
      loop();
      el.scrollIntoView({ behavior: "smooth", block: "start" });
    },
    stop() {
      playback?.pause();
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
    else viewer.show(heading, new Playback(opened.viewer), names);
  };

  const replayLines = async (entry: ReplayEntry, text: string) =>
    joinedReplay(text, entry.parts === undefined ? [] : await api.readReplayParts(entry.folder, entry.serial, entry.parts));

  const row = (entry: ReplayEntry, text: string) => {
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
      h("div.replay-actions", {}, watchButton, entry.parts !== undefined ? share : null, saved),
    );
  };

  const refresh = async () => {
    refreshButton.disabled = true;
    try {
      const [files, records] = await Promise.all([api.readReplays(), api.readRecords()]);
      if (!live) return;
      const { entries, refused } = replayEntries(files, records);
      const texts = new Map(files.map((f) => [`${f.folder}/${f.name}`, f.text]));
      list.replaceChildren(...entries.map((entry) => row(entry, texts.get(entry.key) ?? "")));
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
