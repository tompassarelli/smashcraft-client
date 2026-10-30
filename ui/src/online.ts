// What the Online page shows for a run of Wisp's standalone window (#25): the
// Rust side keeps the command's lines; this reads the join code, the relay
// region and the current step from them.
import type { OnlineState } from "./controller";

/** What the standalone run prints for the relay it uses: `net: hosting through sgp, round trip to it 183 ms`. */
const RELAY_LINE = /^net: (?:hosting|joining) through (\S+), round trip to it (\d+) ms$/;
const STANDALONE_CODE_LINE = /^\S+: join code (\S+) \(/;
const TITLE_PREFIX = /^[A-Za-z0-9 ]+: /;

/** A standalone run's view: its code is the relay code the host gives their opponent, as `sgp-7K2M-QX9D-R3TB-4`. */
export type StandaloneView = {
  code: string | undefined;
  /** The steps to list: every line but the code's and the relay's. */
  steps: string[];
  /** The relay region the match crosses and the round trip to it. */
  region: { name: string; rttMs: number } | undefined;
  busy: boolean;
  failed: boolean;
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
    region: relay ? { name: relay[1]!, rttMs: Number(relay[2]) } : undefined,
    steps: waiting ? [...said, WAITING_FOR_OPPONENT] : failed && said.length === 0 ? [DID_NOT_START] : said,
    busy: state.running,
    failed,
  };
}

/** A relay code: its region, then thirteen letters or digits in groups of four, or a relay's `ADDRESS:PORT/` in front of them. */
const RELAY_CODE = /^(?:[a-z][a-z0-9]{1,7}-|[^\s/]+:\d{1,5}\/)(?:[0-9a-z]{4}-?){3}[0-9a-z]$/i;

export const looksLikeRelayCode = (typed: string) => RELAY_CODE.test(typed.trim());
