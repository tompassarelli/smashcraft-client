// What the Online page shows for a run of `bun wisp online` (#142): the
// Rust side keeps the command's lines; this reads the join code and the
// current step from them.
import type { OnlineState } from "./model";

export type OnlineView = {
  /** The host's join code, once Warcraft III has created the game. */
  code: string | undefined;
  /** The steps to list: every line but the code's own. */
  steps: string[];
  /** A waiting host starts once their opponent has joined. */
  canStartNow: boolean;
  busy: boolean;
  failed: boolean;
};

const CODE_LINE = /^Join code: (\S+)$/;

export function onlineView(state: OnlineState): OnlineView {
  const host = state.mode === "host";
  const code = host ? state.lines.map((line) => CODE_LINE.exec(line)?.[1]).find((found) => found !== undefined) : undefined;
  const loading = state.lines.includes("Loading the match");
  return {
    code,
    steps: state.lines.filter((line) => !CODE_LINE.test(line)),
    canStartNow: host && state.running && code !== undefined && !loading,
    busy: state.running,
    failed: state.finished === false,
  };
}

/** A typed code is worth sending once it has its letters or digits: eight for Battle.net, sixteen for Smashcraft Servers. */
export const looksLikeCode = (typed: string, length = 8) => typed.replace(/[\s-]+/g, "").length === length;

/** Where Host and Join play: Battle.net, or Smashcraft Servers beside it once the player opts in (#26). */
export type Network = "battle.net" | "smashcraft_servers";

/** Tom's decision, 25 October 2026: the exact text the player opts in on. */
export const SMASHCRAFT_SERVERS_OPT_IN = "Smashcraft changes how Warcraft connects while it runs and Blizzard has not approved it.";

export type NetworkView = {
  /** The network Host and Join use. */
  network: Network;
  /** Show the opt-in screen: the player picked Smashcraft Servers without having opted in. */
  askOptIn: boolean;
};

export function networkView(picked: Network, optedIn: boolean | null): NetworkView {
  const servers = picked === "smashcraft_servers";
  return { network: servers && optedIn === true ? "smashcraft_servers" : "battle.net", askOptIn: servers && optedIn !== true };
}
