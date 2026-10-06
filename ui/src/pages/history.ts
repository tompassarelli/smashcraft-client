import { h } from "../dom";
import { refreshHistory } from "../history";
import { type FighterRecord, type IngestResult, type MatchRecord, duration } from "../records";
import { api } from "../tauri";

const MODE: Record<string, string> = { versus: "Versus", practice: "Practice", training: "Training", endless: "Endless" };

const who = (f: FighterRecord) => (f.computer ? "Computer" : f.player ?? f.slot);

function matchRow(record: MatchRecord): HTMLElement {
  const winner = record.fighters.find((f) => f.slot === record.winner);
  const fighters = record.fighters.map((f) =>
    h(`span.fighter${f.slot === record.winner ? ".won" : ""}${f.slot === record.local ? ".you" : ""}`, {}, h("b", {}, f.hero), ` ${who(f)}`),
  );
  const when = record.playedAt > 0 ? new Date(record.playedAt).toLocaleString() : "";
  const result = record.interrupted ? "Ended early" : winner ? `${winner.hero} won` : "Draw";
  return h(
    "li.match",
    {},
    h("div.match-head", {}, h("span.result", {}, result), h("span.muted", {}, `${MODE[record.mode] ?? record.mode} · ${record.stage} · ${duration(record.frames)}`), h("span.muted.when", {}, when)),
    h("div.fighters", {}, ...fighters),
  );
}

function summary(result: IngestResult): string {
  const parts = [result.added.length === 1 ? "1 new match" : `${result.added.length} new matches`];
  if (result.refused.length > 0) parts.push(`${result.refused.length} couldn't be read (${result.refused.map((r) => `${r.file}: ${r.reason}`).join("; ")})`);
  return parts.join(", ");
}

function foldersCard(onChange: () => void): { el: HTMLElement; load: () => Promise<void> } {
  const list = h("ul.folders");
  const input = h("input.folder-input", { type: "text", placeholder: "…/Documents/Warcraft III/CustomMapData" });
  const add = h("button", { type: "submit" }, "Add folder");
  let folders: string[] = [];
  const render = () => {
    list.replaceChildren(
      ...folders.map((folder) => {
        const remove = h("button.quiet", { type: "button" }, "Remove");
        remove.addEventListener("click", () => void save(folders.filter((f) => f !== folder)));
        return h("li", {}, h("span.path", {}, folder), remove);
      }),
    );
    if (folders.length === 0) list.append(h("li.muted", {}, "No folders yet. Add Warcraft III's CustomMapData folder to see your matches."));
  };
  const save = async (next: string[]) => {
    folders = await api.setRecordFolders(next);
    render();
    onChange();
  };
  const form = h("form.add-folder", {}, input, add);
  form.addEventListener("submit", (e) => {
    e.preventDefault();
    const folder = input.value.trim();
    if (folder === "" || folders.includes(folder)) return;
    input.value = "";
    void save([...folders, folder]);
  });
  const el = h(
    "section.card",
    {},
    h("h2", {}, "Where your matches are saved"),
    list,
    form,
    h("p.note", {}, "Smashcraft saves every finished match in Warcraft III's CustomMapData folder. Add each one you play from."),
  );
  return {
    el,
    load: async () => {
      folders = await api.recordFolders();
      render();
    },
  };
}

export function historyPage(root: HTMLElement): () => void {
  const list = h("ol.matches");
  const status = h("p.note");
  const check = h("button.primary", { type: "button" }, "Look for new matches");
  let live = true;
  const refresh = async () => {
    check.disabled = true;
    try {
      const result = await refreshHistory();
      if (!live) return;
      status.textContent = summary(result);
      list.replaceChildren(...result.store.records.map(matchRow));
      if (result.store.records.length === 0) list.append(h("li.muted", {}, "No matches yet. Finished matches show up here."));
    } catch (error) {
      status.textContent = String(error);
    } finally {
      check.disabled = false;
    }
  };
  check.addEventListener("click", () => void refresh());
  const folders = foldersCard(() => void refresh());
  root.append(h("section.card", {}, h("div.card-head", {}, h("h2", {}, "Match history"), check), status, list), folders.el);
  void folders.load().then(refresh);
  return () => {
    live = false;
  };
}
