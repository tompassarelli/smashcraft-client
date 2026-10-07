// What the Online page shows for a run of `bun wisp online` (#142): the
// Rust side keeps the command's lines; this reads the join code and the
// current step from them.
import type { OnlineState } from "./model";

export type OnlineView = {
  /** The host's join code, once Warcraft III has created the game. */
  code: string | undefined;
  /** The steps to list: every line but the code's own. */
  steps: string[];
  /** A waiting host may start before the opponent's ready line arrives. */
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

/** A typed code is worth sending once it has eight letters or digits; the command reads it exactly. */
export const looksLikeCode = (typed: string) => typed.replace(/[\s-]+/g, "").length === 8;
