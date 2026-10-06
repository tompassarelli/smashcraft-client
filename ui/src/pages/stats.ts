import { h } from "../dom";
import { refreshHistory } from "../history";
import { type Stats, computeStats, perMatch, percent } from "../stats";

function table(head: string[], rows: string[][]): HTMLElement {
  return h(
    "table.stats",
    {},
    h("thead", {}, h("tr", {}, ...head.map((cell) => h("th", {}, cell)))),
    h("tbody", {}, ...rows.map((row) => h("tr", {}, ...row.map((cell) => h("td", {}, cell))))),
  );
}

function render(stats: Stats): HTMLElement[] {
  if (stats.matches === 0) return [h("section.card", {}, h("h2", {}, "Stats"), h("p.note", {}, "Play a versus match to see your stats here."))];
  const fighters = table(
    ["Fighter", "Matches", "Win rate", "KOs per match", "Falls per match", "Average damage dealt"],
    stats.fighters.map((f) => [f.hero, String(f.matches), percent(f.wins, f.matches), perMatch(f.kos, f.matches), perMatch(f.falls, f.matches), perMatch(f.dealt, f.combatMatches)]),
  );
  const matchups = table(
    ["Your fighter", "Against", "Matches", "Win rate"],
    stats.matchups.map((m) => [m.hero, m.opponent, String(m.matches), percent(m.wins, m.matches)]),
  );
  return [
    h("section.card", {}, h("h2", {}, `Your fighters · ${stats.matches} versus ${stats.matches === 1 ? "match" : "matches"}`), fighters),
    h("section.card", {}, h("h2", {}, "Matchups"), matchups),
  ];
}

export function statsPage(root: HTMLElement): () => void {
  let live = true;
  void refreshHistory()
    .then((result) => live && root.replaceChildren(...render(computeStats(result.store.records))))
    .catch((error) => live && root.replaceChildren(h("p.note", {}, String(error))));
  return () => {
    live = false;
  };
}
