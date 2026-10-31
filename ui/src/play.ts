import { h } from "./dom";
import type { PlayState } from "./controller";
import { stepsAfterError } from "./play-steps";
import type { Api } from "./api";

/** The header's Play button and the steps of the current run beneath it. */
export function playButton(api: Api): HTMLElement {
  const button = h("button.play", { type: "button" }, "Play");
  const steps = h("ol.play-steps");
  const panel = h("div.play-panel", { hidden: true }, h("h3", {}, "Getting you into a match"), steps);
  const render = (state: PlayState) => {
    button.disabled = !state.available || state.running;
    button.textContent = state.running ? "Starting…" : "Play";
    button.title = state.available ? "Start Warcraft III on the current Smashcraft map" : "Put a Smashcraft map in Warcraft III's Maps/00-Smashcraft folder first.";
    if (state.lines.length || state.running) panel.hidden = false;
    steps.replaceChildren(...state.lines.map((line) => h("li", {}, line)));
    panel.classList.toggle("failed", state.finished === false);
  };
  button.addEventListener("click", async () => {
    try {
      render(await api.play());
    } catch (error) {
      const state = await api.playState().catch(() => undefined);
      render({ available: true, running: false, ...state, map: null, lines: stepsAfterError(state?.lines ?? [], error), finished: false });
    }
  });
  panel.addEventListener("click", (e) => {
    if (e.target === panel) panel.hidden = true;
  });
  void api.playState().then(render);
  void api.onPlay(render);
  return h("div.play-wrap", {}, button, panel);
}
