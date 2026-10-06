// The stats page's numbers, from the history store. Each record counts from
// its writer's side: the fighter the player who wrote it played. Only versus
// matches count; practice, training, endless and observed matches don't.
import { type MatchRecord, writerOf } from "./records";

export type FighterStats = {
  hero: string;
  matches: number;
  wins: number;
  kos: number;
  falls: number;
  /** Damage dealt over the matches whose records carry combat stats. */
  dealt: number;
  combatMatches: number;
};

export type MatchupStats = { hero: string; opponent: string; matches: number; wins: number };

export type Stats = { matches: number; fighters: FighterStats[]; matchups: MatchupStats[] };

/** Win rate, KOs, falls and damage per fighter and per matchup, most played first. */
export function computeStats(records: readonly MatchRecord[]): Stats {
  const fighters = new Map<string, FighterStats>();
  const matchups = new Map<string, MatchupStats>();
  let matches = 0;
  for (const record of records) {
    const own = writerOf(record);
    if (record.mode !== "versus" || own === undefined) continue;
    matches++;
    const won = record.winner === own.slot;
    const stats = fighters.get(own.hero) ?? { hero: own.hero, matches: 0, wins: 0, kos: 0, falls: 0, dealt: 0, combatMatches: 0 };
    fighters.set(own.hero, stats);
    stats.matches++;
    if (won) stats.wins++;
    stats.kos += own.kos;
    stats.falls += own.falls;
    if (own.combat !== undefined) {
      stats.dealt += own.combat.dealt;
      stats.combatMatches++;
    }
    for (const other of record.fighters) {
      if (other === own) continue;
      const key = `${own.hero}\n${other.hero}`;
      const matchup = matchups.get(key) ?? { hero: own.hero, opponent: other.hero, matches: 0, wins: 0 };
      matchups.set(key, matchup);
      matchup.matches++;
      if (won) matchup.wins++;
    }
  }
  const byPlayed = <T extends { matches: number }>(a: T, b: T) => b.matches - a.matches;
  return { matches, fighters: [...fighters.values()].sort(byPlayed), matchups: [...matchups.values()].sort(byPlayed) };
}

/** A whole-number percentage, or "–" with nothing to divide. */
export const percent = (part: number, whole: number) => (whole > 0 ? `${Math.round((part * 100) / whole)}%` : "–");

/** A per-match average to one decimal place, or "–" with nothing to divide. */
export const perMatch = (total: number, matches: number) => (matches > 0 ? (total / matches).toFixed(1) : "–");
