import { h } from "../dom";
import type { OnlineState } from "../controller";
import { looksLikeCode, looksLikeRelayCode, type Network, networkView, type OnlineView, onlineView, SMASHCRAFT_SERVERS_OPT_IN, type StandaloneView, standaloneView } from "../online";
import { onlineApi } from "../tauri-online";

const EMPTY: OnlineState = { available: false, running: false, map: null, mode: null, lines: [], finished: null };

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
  { network: "standalone", label: "Smashcraft window", hint: "Smashcraft's own window through Smashcraft's relay; Warcraft III stays closed" },
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
type Shown = { network: Network; available: boolean; state: OnlineState; view: OnlineView | StandaloneView };

const HOST_NOTE: Record<Network, string> = {
  standalone: "Opens Smashcraft's window and a room on the nearest relay. Give your opponent the code.",
  smashcraft_servers: "Start Warcraft III with Play and sign in, then host. Give your opponent the code.",
  "battle.net": "Open Warcraft III and sign in, then host. Give your opponent the code; press Start now once they have joined.",
};

const JOIN_NOTE: Record<Network, string> = {
  standalone: "Type the code your opponent gave you.",
  smashcraft_servers: "Start Warcraft III with Play and sign in, then type the code your opponent gave you.",
  "battle.net": "Open Warcraft III and sign in, then type the code your opponent gave you.",
};

function regionLine(view: OnlineView | StandaloneView): HTMLElement | null {
  return "region" in view && view.region ? h("p.note", {}, `Region ${view.region.name}, ${view.region.rttMs} ms away`) : null;
}

function playGrid({ network, available, state, view }: Shown, codeInput: HTMLElement, act: { host: () => void; join: () => void; startNow: () => void }): HTMLElement {
  const hosting = state.mode === "host" && (view.busy || (network !== "standalone" && view.code !== undefined));
  const hostBody = hosting && view.code !== undefined ? [codeBox(view, network === "battle.net" ? act.startNow : undefined), regionLine(view)] : [h("button.primary.online-go", { type: "button", onclick: act.host, disabled: !available || view.busy }, "Host")];
  return h(
    "div.grid",
    {},
    h("section.card", {}, h("h2", {}, "Host a game"), h("p.note", {}, HOST_NOTE[network]), ...hostBody.filter((part) => part !== null)),
    h(
      "section.card",
      {},
      h("h2", {}, "Join a game"),
      h("p.note", {}, JOIN_NOTE[network]),
      h("div.online-join", {}, codeInput, h("button.primary", { type: "button", onclick: act.join, disabled: !available || view.busy }, "Join")),
      ...[state.mode === "join" && view.busy ? regionLine(view) : null].filter((part) => part !== null),
    ),
  );
}

function notes({ network, available, view }: Shown, choice: boolean | null, problem: string | undefined, setUp: () => void): HTMLElement[] {
  return [
    ...(!available ? [h("p.note", {}, "Online play needs Smashcraft's tools (SMASHCRAFT_TS).")] : []),
    ...(problem ? [h("p.problem", {}, problem)] : []),
    ...(network === "battle.net" && choice === true && !view.busy ? [h("button.quiet", { type: "button", onclick: setUp, title: "After a Warcraft III update, or if hosting can't find Warcraft III's menus" }, "Set up Warcraft III's menus again")] : []),
  ];
}

const PLACEHOLDER: Record<Network, string> = { standalone: "sgp-ABCD-EFGH-JKMN-P", smashcraft_servers: "0123456789abcdef", "battle.net": "ABCD-EFGH" };

const codeReady = (network: Network, typed: string) => (network === "standalone" ? looksLikeRelayCode(typed) : looksLikeCode(typed, network === "smashcraft_servers" ? 16 : 8));

/** Follows the three runs' events; the returned calls stop following, and a page closed first stops at once. */
function follow(live: () => boolean, handlers: Record<Network, (state: OnlineState) => void>): (() => void)[] {
  const stops: (() => void)[] = [];
  for (const listen of [onlineApi.onStandalone(handlers.standalone), onlineApi.onOnline(handlers["battle.net"]), onlineApi.onRelay(handlers.smashcraft_servers)]) {
    void listen.then((unlisten) => {
      if (live()) stops.push(unlisten);
      else unlisten();
    });
  }
  return stops;
}

export function onlinePage(root: HTMLElement): () => void {
  let live = true;
  const runs: Record<Network, OnlineState> = { standalone: EMPTY, "battle.net": EMPTY, smashcraft_servers: EMPTY };
  let choice: boolean | null = null;
  let servers: boolean | null = null;
  let picked: Network = "standalone";
  let problem: string | undefined;

  const codeInput = h("input.code-input", { type: "text", placeholder: PLACEHOLDER.standalone, maxLength: 40, spellcheck: false, autocomplete: "off" });
  const inUse = () => networkView(picked, servers).network;

  const act = (run: () => Promise<OnlineState | void>) => async () => {
    problem = undefined;
    const network = inUse();
    try {
      const next = await run();
      if (next) runs[network] = next;
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
  const notNow = act(async () => void (picked = "standalone"));
  const pick = (network: Network) => void act(async () => void (picked = network))();
  const by = <T>(options: Record<Network, () => T>) => options[inUse()]();
  const host = act(() => by({ standalone: onlineApi.standaloneHost, smashcraft_servers: onlineApi.relayHost, "battle.net": onlineApi.onlineHost }));
  const join = act(() => by({ standalone: () => onlineApi.standaloneJoin(codeInput.value), smashcraft_servers: () => onlineApi.relayJoin(codeInput.value), "battle.net": () => onlineApi.onlineJoin(codeInput.value) }));
  const startNow = act(async () => void (await onlineApi.onlineStartNow()));
  const cancel = act(() => by({ standalone: onlineApi.standaloneCancel, smashcraft_servers: onlineApi.relayCancel, "battle.net": onlineApi.onlineCancel }));

  const render = () => {
    if (!live) return;
    const { network, askOptIn } = networkView(picked, servers);
    const state = runs[network];
    const available = network !== "smashcraft_servers" ? state.available : true;
    const view = network === "standalone" ? standaloneView(state) : onlineView(state);
    codeInput.placeholder = PLACEHOLDER[network];
    root.replaceChildren(
      networkCard(network, pick),
      ...(askOptIn ? [optInCard(optIn, notNow)] : []),
      ...(network === "battle.net" && choice !== true ? [askCard(choice, setUp, decline)] : []),
      playGrid({ network, available, state, view }, codeInput, { host, join, startNow }),
      ...[stepsCard(state, view, cancel)].filter((card) => card !== null),
      ...notes({ network, available, state, view }, choice, problem, setUp),
    );
  };

  codeInput.addEventListener("keydown", (event) => {
    const network = inUse();
    if (event.key === "Enter" && codeReady(network, codeInput.value) && !runs[network].running) void join();
  });

  void Promise.all([onlineApi.onlineState(), onlineApi.menuPageChoice(), onlineApi.relayState(), onlineApi.smashcraftServersChoice(), onlineApi.standaloneState()]).then(([current, chosen, relayed, optedIn, standalone]) => {
    runs["battle.net"] = current;
    choice = chosen;
    runs.smashcraft_servers = relayed;
    servers = optedIn;
    runs.standalone = standalone;
    render();
  });
  const changed = (network: Network) => (next: OnlineState) => void ((runs[network] = next), render());
  const stops = follow(() => live, { standalone: changed("standalone"), "battle.net": changed("battle.net"), smashcraft_servers: changed("smashcraft_servers") });
  render();
  return () => {
    live = false;
    stops.forEach((stop) => stop());
  };
}
