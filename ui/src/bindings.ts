import { controlLabel, type Binding, type Control } from "./controller";

export type Line = { tag: string; text: string; kind: "fight" | "menus" | "none" };

export function callouts(fight: Binding[], menus: Binding[]): Map<string, Line[]> {
  const of = (bindings: Binding[], control: Control) => bindings.find((b) => b.control === control)?.action;
  const button = (control: Control): Line[] => {
    const name = controlLabel(control);
    const f = of(fight, control);
    const m = of(menus, control);
    const lines: Line[] = [];
    if (f) lines.push({ tag: name, text: f, kind: "fight" });
    if (m && m !== f) lines.push({ tag: f ? "Menus" : `${name} · Menus`, text: m, kind: "menus" });
    return lines.length ? lines : [{ tag: name, text: "Not used", kind: "none" }];
  };
  // Four directions doing the same thing read as one line.
  const directions = (prefix: string, bindings: Binding[], kind: Line["kind"], tag: string): Line[] => {
    const arrows: [string, string][] = [["up", "↑"], ["down", "↓"], ["left", "←"], ["right", "→"]];
    const actions = arrows.map(([d, arrow]) => [arrow, of(bindings, `${prefix}${d}` as Control)] as const);
    if (actions.every(([, a]) => a === undefined)) return [];
    if (actions.every(([, a]) => a === actions[0]![1])) return [{ tag, text: actions[0]![1]!, kind }];
    return actions.map(([arrow, a]) => ({ tag: `${tag} ${arrow}`.trim(), text: a ?? "Not used", kind: a ? kind : "none" }));
  };
  const stick = (name: string, prefix: string, click?: Control): Line[] => {
    const lines: Line[] = [{ tag: name, text: "", kind: "fight" }, ...directions(prefix, fight, "fight", ""), ...directions(prefix, menus, "menus", "Menus")];
    if (click) lines.push(...button(click).map((l) => ({ ...l, tag: l.tag.replace(controlLabel(click), "Click") })));
    return lines;
  };
  const map = new Map<string, Line[]>();
  for (const c of ["lt", "rt", "lb", "rb", "back", "start", "a", "b", "x", "y"] as const) map.set(c, button(c));
  map.set("left", stick("Left stick", "left_", "left_stick"));
  map.set("right", stick("Right stick", "right_"));
  const dpad = stick("D-pad", "dpad_");
  map.set("dpad", dpad.length > 1 ? dpad : [{ tag: "D-pad", text: "Not used", kind: "none" }]);
  return map;
}
