import { h } from "../dom";
import type { OnlineState } from "../model";
import { looksLikeCode, onlineView } from "../online";
import { api } from "../tauri";

const EMPTY: OnlineState = { available: false, running: false, mode: null, lines: [], finished: null };

/** Asked once: online play needs Smashcraft's page in Warcraft III's menus. */
function askCard(choice: boolean | null, setUp: () => void, decline: () => void): HTMLElement {
  return h(
    "section.card",
    {},
    h("h2", {}, "Set up online play"),
    h("p", {}, "To host a game or join a friend's with a code, Smashcraft adds its page to Warcraft III's menus, as other Warcraft III community apps do. Battle.net sees the same requests as your own clicks."),
    h("p.note", {}, "Close Warcraft III and Battle.net first."),
    h(
      "div.online-actions",
      {},
      h("button.primary", { type: "button", onclick: setUp }, "Set up online play"),
      choice === null && h("button.quiet", { type: "button", onclick: decline }, "Not now"),
    ),
  );
}

export function onlinePage(root: HTMLElement): () => void {
  let live = true;
  let state = EMPTY;
  let choice: boolean | null = null;
  let problem: string | undefined;

  const codeInput = h("input.code-input", { type: "text", placeholder: "ABCD-EFGH", maxLength: 12, spellcheck: false, autocomplete: "off" });

  const act = (run: () => Promise<OnlineState | void>) => async () => {
    problem = undefined;
    try {
      const next = await run();
      if (next) state = next;
    } catch (error) {
      problem = String(error);
    }
    render();
  };
  const setUp = act(async () => {
    choice = true;
    return api.onlineSetup();
  });
  const decline = act(async () => {
    await api.declineMenuPage();
    choice = false;
  });
  const host = act(() => api.onlineHost());
  const join = act(() => api.onlineJoin(codeInput.value));
  const startNow = act(async () => void (await api.onlineStartNow()));
  const cancel = act(() => api.onlineCancel());

  const render = () => {
    if (!live) return;
    const view = onlineView(state);
    const hosting = state.mode === "host" && (view.busy || view.code !== undefined);
    const steps = view.steps.length > 0 || view.busy
      ? h(
        "section.card",
        { className: view.failed ? "card online-failed" : "card" },
        h("div.card-head", {}, h("h2", {}, state.mode === "setup" ? "Setting up" : state.mode === "join" ? "Joining" : "Hosting"), view.busy && h("button.quiet", { type: "button", onclick: cancel }, "Cancel")),
        h("ol.play-steps", {}, ...view.steps.map((line) => h("li", {}, line))),
      )
      : null;
    const codeCard = hosting && view.code !== undefined
      ? h(
        "div.code-box",
        {},
        h("div.code", {}, view.code),
        h("button", { type: "button", onclick: () => void navigator.clipboard?.writeText(view.code!) }, "Copy"),
        view.canStartNow && h("button.primary", { type: "button", onclick: startNow, title: "Start once your opponent has joined" }, "Start now"),
      )
      : null;
    root.replaceChildren(
      ...(choice !== true ? [askCard(choice, setUp, decline)] : []),
      h(
        "div.grid",
        {},
        h(
          "section.card",
          {},
          h("h2", {}, "Host a game"),
          h("p.note", {}, "Open Warcraft III and sign in, then host. Give your opponent the code; press Start now once they have joined."),
          codeCard ?? h("button.primary.online-go", { type: "button", onclick: host, disabled: !state.available || view.busy }, "Host"),
        ),
        h(
          "section.card",
          {},
          h("h2", {}, "Join a game"),
          h("p.note", {}, "Open Warcraft III and sign in, then type the code your opponent gave you."),
          h("div.online-join", {}, codeInput, h("button.primary", { type: "button", onclick: join, disabled: !state.available || view.busy }, "Join")),
        ),
      ),
      ...(steps ? [steps] : []),
      ...(!state.available ? [h("p.note", {}, "Online play needs Smashcraft's tools (SMASHCRAFT_TS).")] : []),
      ...(problem ? [h("p.problem", {}, problem)] : []),
      ...(choice === true && !view.busy ? [h("button.quiet", { type: "button", onclick: setUp, title: "After a Warcraft III update, or if hosting can't find Warcraft III's menus" }, "Set up Warcraft III's menus again")] : []),
    );
  };

  codeInput.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && looksLikeCode(codeInput.value) && !state.running) void join();
  });

  let stop: (() => void) | undefined;
  void Promise.all([api.onlineState(), api.menuPageChoice()]).then(([current, chosen]) => {
    state = current;
    choice = chosen;
    render();
  });
  void api.onOnline((next) => {
    state = next;
    render();
  }).then((unlisten) => {
    if (live) stop = unlisten;
    else unlisten();
  });
  render();
  return () => {
    live = false;
    stop?.();
  };
}
