import { h } from "../dom";
import type { OnlineState } from "../controller";
import { looksLikeRelayCode, type StandaloneView, standaloneView } from "../online";
import { onlineApi } from "../tauri-online";

const EMPTY: OnlineState = { available: false, running: false, map: null, mode: null, lines: [], finished: null };

function stepsCard(state: OnlineState, view: StandaloneView, cancel: () => void): HTMLElement | null {
  if (view.steps.length === 0 && !view.busy) return null;
  return h(
    "section.card",
    { className: view.failed ? "card online-failed" : "card" },
    h("div.card-head", {}, h("h2", {}, state.mode === "join" ? "Joining" : "Hosting"), view.busy && h("button.quiet", { type: "button", onclick: cancel }, "Cancel")),
    h("ol.play-steps", {}, ...view.steps.map((line) => h("li", {}, line))),
  );
}

function codeBox(view: StandaloneView): HTMLElement {
  return h("div.code-box", {}, h("div.code", {}, view.code), h("button", { type: "button", onclick: () => void navigator.clipboard?.writeText(view.code!) }, "Copy"));
}

function regionLine(view: StandaloneView): HTMLElement | null {
  return view.region ? h("p.note", {}, `Region ${view.region.name}, ${view.region.rttMs} ms away`) : null;
}

function playGrid(state: OnlineState, view: StandaloneView, codeInput: HTMLElement, act: { host: () => void; join: () => void }): HTMLElement {
  const hosting = state.mode === "host" && view.busy && view.code !== undefined;
  const hostBody = hosting ? [codeBox(view), regionLine(view)] : [h("button.primary.online-go", { type: "button", onclick: act.host, disabled: !state.available || view.busy }, "Host")];
  return h(
    "div.grid",
    {},
    h("section.card", {}, h("h2", {}, "Host a game"), h("p.note", {}, "Opens Smashcraft's window and a room on the nearest relay. Give your opponent the code."), ...hostBody.filter((part) => part !== null)),
    h(
      "section.card",
      {},
      h("h2", {}, "Join a game"),
      h("p.note", {}, "Type the code your opponent gave you."),
      h("div.online-join", {}, codeInput, h("button.primary", { type: "button", onclick: act.join, disabled: !state.available || view.busy }, "Join")),
      ...[state.mode === "join" && view.busy ? regionLine(view) : null].filter((part) => part !== null),
    ),
  );
}

/** Online play is Smashcraft's own window through Wisp's relay; Warcraft III stays closed. */
export function onlinePage(root: HTMLElement): () => void {
  let live = true;
  let run: OnlineState = EMPTY;
  let problem: string | undefined;
  let stop: (() => void) | undefined;

  const codeInput = h("input.code-input", { type: "text", placeholder: "sgp-ABCD-EFGH-JKMN-P", maxLength: 40, spellcheck: false, autocomplete: "off" });

  const act = (call: () => Promise<OnlineState | void>) => async () => {
    problem = undefined;
    try {
      const next = await call();
      if (next) run = next;
    } catch (error) {
      problem = String(error);
    }
    render();
  };
  const host = act(onlineApi.standaloneHost);
  const join = act(() => onlineApi.standaloneJoin(codeInput.value));
  const cancel = act(onlineApi.standaloneCancel);

  const render = () => {
    if (!live) return;
    const view = standaloneView(run);
    root.replaceChildren(
      playGrid(run, view, codeInput, { host, join }),
      ...[stepsCard(run, view, cancel)].filter((card) => card !== null),
      ...(!run.available ? [h("p.note", {}, "Online play needs Smashcraft's tools (SMASHCRAFT_TS).")] : []),
      ...(problem ? [h("p.problem", {}, problem)] : []),
    );
  };

  codeInput.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && looksLikeRelayCode(codeInput.value) && !run.running) void join();
  });

  void onlineApi.standaloneState().then((current) => {
    run = current;
    render();
  });
  void onlineApi
    .onStandalone((next) => {
      run = next;
      render();
    })
    .then((unlisten) => {
      if (live) stop = unlisten;
      else unlisten();
    });
  render();
  return () => {
    live = false;
    stop?.();
  };
}
