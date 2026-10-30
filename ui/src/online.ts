// What the Online page shows for a run of `bun wisp online` (#142) or of Wisp's
// standalone window (#25): the Rust side keeps the command's lines; this reads
// the join code, the relay region and the current step from them.
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

/** What the standalone run prints for the relay it uses: `net: hosting through sgp, round trip to it 183 ms`. */
const RELAY_LINE = /^net: (?:hosting|joining) through (\S+), round trip to it (\d+) ms$/;
const STANDALONE_CODE_LINE = /^\S+: join code (\S+) \(/;
const TITLE_PREFIX = /^[A-Za-z0-9 ]+: /;

/** A standalone run's view: its code is the relay code the host gives their opponent, as `sgp-7K2M-QX9D-R3TB-4`, and nobody starts it early. */
export type StandaloneView = OnlineView & {
  /** The relay region the match crosses and the round trip to it. */
  region: { name: string; rttMs: number } | undefined;
};

const WINDOW_OPENED = /standalone window opened/;

export const WAITING_FOR_OPPONENT = "Waiting for your opponent to join";
export const DID_NOT_START = "The match didn't start. The client's log folder has standalone.log.";

export function standaloneView(state: OnlineState): StandaloneView {
  const host = state.mode === "host";
  const code = host ? state.lines.map((line) => STANDALONE_CODE_LINE.exec(line)?.[1]).find((found) => found !== undefined) : undefined;
  const relay = state.lines.map((line) => RELAY_LINE.exec(line)).find((found) => found !== null);
  const said = state.lines.filter((line) => !STANDALONE_CODE_LINE.test(line) && !RELAY_LINE.test(line)).map((line) => line.replace(TITLE_PREFIX, ""));
  const waiting = state.running && code !== undefined && !state.lines.some((line) => WINDOW_OPENED.test(line));
  const failed = state.finished === false;
  return {
    code,
    canStartNow: false,
    region: relay ? { name: relay[1]!, rttMs: Number(relay[2]) } : undefined,
    steps: waiting ? [...said, WAITING_FOR_OPPONENT] : failed && said.length === 0 ? [DID_NOT_START] : said,
    busy: state.running,
    failed,
  };
}

/** A relay code: its region, then thirteen letters or digits in groups of four, or a relay's `ADDRESS:PORT/` in front of them. */
const RELAY_CODE = /^(?:[a-z][a-z0-9]{1,7}-|[^\s/]+:\d{1,5}\/)(?:[0-9a-z]{4}-?){3}[0-9a-z]$/i;

export const looksLikeRelayCode = (typed: string) => RELAY_CODE.test(typed.trim());

/** Where Host and Join play: Smashcraft's own window through Wisp's relay, Battle.net, or Smashcraft Servers beside it once the player opts in (#26). */
export type Network = "standalone" | "battle.net" | "smashcraft_servers";

/** Tom's decision, 25 October 2026: the exact text the player opts in on. */
export const SMASHCRAFT_SERVERS_OPT_IN = "Smashcraft changes how Warcraft connects while it runs and Blizzard has not approved it.";

export type NetworkView = {
  /** The network Host and Join use. */
  network: Network;
  /** Show the opt-in screen: the player picked Smashcraft Servers without having opted in. */
  askOptIn: boolean;
};

export function networkView(picked: Network, optedIn: boolean | null): NetworkView {
  if (picked === "standalone") return { network: "standalone", askOptIn: false };
  const servers = picked === "smashcraft_servers";
  return { network: servers && optedIn === true ? "smashcraft_servers" : "battle.net", askOptIn: servers && optedIn !== true };
}
