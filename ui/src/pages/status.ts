import { h } from "../dom";
import type { ControllerState } from "../controller";
import type { Api } from "../api";

const KEYBOARD_NOTE = "Keyboard play needs no controller. Set your keys in Warcraft III's own Controls screen (WASD, Box or Custom).";

export function statusCard(api: Api): { el: HTMLElement; render: (s: ControllerState) => void; setMap: (map: string | null) => void } {
  const rows = h("div.rows");
  const keyboard = h("p.note", { hidden: true }, KEYBOARD_NOTE);
  let shown: ControllerState | undefined;
  let map: string | null = null;
  const problem = h("p.problem", { hidden: true });
  const turnOn = h("button.primary", { type: "button" }, "Turn on controller support");
  const offNote = h(
    "p.note",
    {},
    "Controller support lets your gamepad play Warcraft III. It keeps running in the background, also when this window is closed.",
  );
  const off = h("div.turn-on", { hidden: true }, offNote, turnOn);
  turnOn.addEventListener("click", async () => {
    turnOn.disabled = true;
    try {
      render(await api.turnOnController());
    } finally {
      turnOn.disabled = false;
    }
  });
  const render = (s: ControllerState) => {
    shown = s;
    const nothingRunning = s.link === "connected" && !s.snapshot.game;
    rows.replaceChildren(
      ...s.view.rows.map((row, i) => {
        const text = i === 2 && nothingRunning && map ? `${map} is ready. Press Play to start a match.` : row.text;
        return h(`div.row.${row.light}`, {}, h("span.light"), h("span.label", {}, row.label), h("span.text", {}, text));
      }),
    );
    keyboard.hidden = s.link === "connected" && !!s.snapshot.pad;
    problem.hidden = !s.view.problem;
    problem.textContent = s.view.problem ?? "";
    off.hidden = s.link !== "off";
    turnOn.textContent = s.wanted ? "Try again" : "Turn on controller support";
  };
  const setMap = (name: string | null) => {
    map = name;
    if (shown) render(shown);
  };
  return { el: h("section.card.status", {}, h("h2", {}, "Status"), rows, keyboard, problem, off), render, setMap };
}
