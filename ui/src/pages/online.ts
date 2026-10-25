import { h } from "../dom";
import type { OnlineState } from "../model";
import { looksLikeCode, type Network, networkView, type OnlineView, onlineView, SMASHCRAFT_SERVERS_OPT_IN } from "../online";
import { onlineApi } from "../tauri-online";

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

/** Smashcraft Servers stays off until the player opts in here, on its exact text. */
function optInCard(optIn: () => void, notNow: () => void): HTMLElement {
  return h(
    "section.card",
    {},
    h("h2", {}, "Smashcraft Servers"),
    h("p", {}, SMASHCRAFT_SERVERS_OPT_IN),
    h("p.note", {}, "Smashcraft Servers stays off until you opt in. Battle.net works without it."),
    h(
      "div.online-actions",
      {},
      h("button.primary", { type: "button", onclick: optIn }, "Opt in"),
      h("button.quiet", { type: "button", onclick: notNow }, "Not now"),
    ),
  );
}

const NETWORKS: { network: Network; label: string; hint: string }[] = [
  { network: "battle.net", label: "Battle.net", hint: "A private Battle.net game with a join code" },
  { network: "smashcraft_servers", label: "Smashcraft Servers", hint: "Smashcraft's relay, once you opt in" },
];

function networkCard(network: Network, pick: (network: Network) => void): HTMLElement {
  const options = NETWORKS.map(({ network: option, label, hint }) => {
    const input = h("input", { type: "radio", name: "network", value: option, checked: option === network, onchange: () => pick(option) });
    return h("label.choice", { title: hint }, input, h("span", {}, label), h("small", {}, hint));
  });
  return h("section.card", {}, h("h2", {}, "Play on"), h("div.choices", {}, ...options));
}

function stepsCard(state: OnlineState, view: OnlineView, cancel: () => void): HTMLElement | null {
  if (view.steps.length === 0 && !view.busy) return null;
  return h(
    "section.card",
    { className: view.failed ? "card online-failed" : "card" },
    h("div.card-head", {}, h("h2", {}, state.mode === "setup" ? "Setting up" : state.mode === "join" ? "Joining" : "Hosting"), view.busy && h("button.quiet", { type: "button", onclick: cancel }, "Cancel")),
    h("ol.play-steps", {}, ...view.steps.map((line) => h("li", {}, line))),
  );
}

function codeBox(view: OnlineView, startNow: (() => void) | undefined): HTMLElement {
  return h(
    "div.code-box",
    {},
    h("div.code", {}, view.code),
    h("button", { type: "button", onclick: () => void navigator.clipboard?.writeText(view.code!) }, "Copy"),
    startNow && view.canStartNow && h("button.primary", { type: "button", onclick: startNow, title: "Start once your opponent has joined" }, "Start now"),
  );
}

/** What the page shows for the network in use: its run, whether it can run here, and the run's view. */
type Shown = { relayed: boolean; available: boolean; state: OnlineState; view: OnlineView };

function playGrid({ relayed, available, state, view }: Shown, codeInput: HTMLElement, act: { host: () => void; join: () => void; startNow: () => void }): HTMLElement {
  const hosting = state.mode === "host" && (view.busy || view.code !== undefined);
  return h(
    "div.grid",
    {},
    h(
      "section.card",
      {},
      h("h2", {}, "Host a game"),
      h("p.note", {}, relayed ? "Start Warcraft III with Play and sign in, then host. Give your opponent the code." : "Open Warcraft III and sign in, then host. Give your opponent the code; press Start now once they have joined."),
      hosting && view.code !== undefined ? codeBox(view, relayed ? undefined : act.startNow) : h("button.primary.online-go", { type: "button", onclick: act.host, disabled: !available || view.busy }, "Host"),
    ),
    h(
      "section.card",
      {},
      h("h2", {}, "Join a game"),
      h("p.note", {}, relayed ? "Start Warcraft III with Play and sign in, then type the code your opponent gave you." : "Open Warcraft III and sign in, then type the code your opponent gave you."),
      h("div.online-join", {}, codeInput, h("button.primary", { type: "button", onclick: act.join, disabled: !available || view.busy }, "Join")),
    ),
  );
}

function notes({ relayed, available, view }: Shown, choice: boolean | null, problem: string | undefined, setUp: () => void): HTMLElement[] {
  return [
    ...(!available ? [h("p.note", {}, "Online play needs Smashcraft's tools (SMASHCRAFT_TS).")] : []),
    ...(problem ? [h("p.problem", {}, problem)] : []),
    ...(!relayed && choice === true && !view.busy ? [h("button.quiet", { type: "button", onclick: setUp, title: "After a Warcraft III update, or if hosting can't find Warcraft III's menus" }, "Set up Warcraft III's menus again")] : []),
  ];
}

/** Follows both runs' events; the returned calls stop following, and a page closed first stops at once. */
function follow(live: () => boolean, onBattle: (state: OnlineState) => void, onRelay: (state: OnlineState) => void): (() => void)[] {
  const stops: (() => void)[] = [];
  for (const listen of [onlineApi.onOnline(onBattle), onlineApi.onRelay(onRelay)]) {
    void listen.then((unlisten) => {
      if (live()) stops.push(unlisten);
      else unlisten();
    });
  }
  return stops;
}

export function onlinePage(root: HTMLElement): () => void {
  let live = true;
  let battle = EMPTY;
  let relay = EMPTY;
  let choice: boolean | null = null;
  let servers: boolean | null = null;
  let picked: Network = "battle.net";
  let problem: string | undefined;

  const codeInput = h("input.code-input", { type: "text", placeholder: "ABCD-EFGH", maxLength: 24, spellcheck: false, autocomplete: "off" });
  const onServers = () => networkView(picked, servers).network === "smashcraft_servers";

  const act = (run: () => Promise<OnlineState | void>) => async () => {
    problem = undefined;
    try {
      const next = await run();
      if (next && onServers()) relay = next;
      else if (next) battle = next;
    } catch (error) {
      problem = String(error);
    }
    render();
  };
  const setUp = act(async () => {
    choice = true;
    return onlineApi.onlineSetup();
  });
  const decline = act(async () => {
    await onlineApi.declineMenuPage();
    choice = false;
  });
  const optIn = act(async () => {
    await onlineApi.optInSmashcraftServers();
    servers = true;
  });
  const notNow = act(async () => void (picked = "battle.net"));
  const pick = (network: Network) => void act(async () => void (picked = network))();
  const host = act(() => (onServers() ? onlineApi.relayHost() : onlineApi.onlineHost()));
  const join = act(() => (onServers() ? onlineApi.relayJoin(codeInput.value) : onlineApi.onlineJoin(codeInput.value)));
  const startNow = act(async () => void (await onlineApi.onlineStartNow()));
  const cancel = act(() => (onServers() ? onlineApi.relayCancel() : onlineApi.onlineCancel()));

  const render = () => {
    if (!live) return;
    const { network, askOptIn } = networkView(picked, servers);
    const relayed = network === "smashcraft_servers";
    const state = relayed ? relay : battle;
    const available = relayed || state.available;
    const view = onlineView(state);
    codeInput.placeholder = relayed ? "0123456789abcdef" : "ABCD-EFGH";
    root.replaceChildren(
      networkCard(network, pick),
      ...(askOptIn ? [optInCard(optIn, notNow)] : []),
      ...(!relayed && choice !== true ? [askCard(choice, setUp, decline)] : []),
      playGrid({ relayed, available, state, view }, codeInput, { host, join, startNow }),
      ...[stepsCard(state, view, cancel)].filter((card) => card !== null),
      ...notes({ relayed, available, state, view }, choice, problem, setUp),
    );
  };

  codeInput.addEventListener("keydown", (event) => {
    const relayed = onServers();
    if (event.key === "Enter" && looksLikeCode(codeInput.value, relayed ? 16 : 8) && !(relayed ? relay : battle).running) void join();
  });

  void Promise.all([onlineApi.onlineState(), onlineApi.menuPageChoice(), onlineApi.relayState(), onlineApi.smashcraftServersChoice()]).then(([current, chosen, relayed, optedIn]) => {
    battle = current;
    choice = chosen;
    relay = relayed;
    servers = optedIn;
    render();
  });
  const stops = follow(() => live, (next) => void ((battle = next), render()), (next) => void ((relay = next), render()));
  render();
  return () => {
    live = false;
    stops.forEach((stop) => stop());
  };
}
