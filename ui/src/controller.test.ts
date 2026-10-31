import { expect, test } from "bun:test";
import { callouts } from "./bindings";
import { KEYS, pressed, pressFromValue, pressLabel, pressOptions, pressValue, stickDot, type Binding, type Control, type InputView, type Press } from "./controller";
import { resolve, type Route } from "./router";

/** mulberry32: the same cases on every run. */
const random = (seed: number) => () => {
  seed = (seed + 0x6d2b79f5) | 0;
  let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
  t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
};

test("button bits follow wc3-controller v0.2.1 model::Button (bit = 1 << discriminant: A, B, X, Y, LB, RB, Start, ...)", () => {
  const input: InputView = { left: [0, 0], right: [0, 0], lt: 0, rt: 0, buttons: 1 | (1 << 6) };
  expect(pressed(input, "a")).toBe(true);
  expect(pressed(input, "start")).toBe(true);
  expect(pressed(input, "b")).toBe(false);
});

test("a stick's dot stays inside the unit circle, keeps its direction with y down, and only full pushes reach the rim (seed 3, 2000 cases)", () => {
  const next = random(3);
  const axis = () => Math.floor(next() * 65536) - 32768;
  for (let i = 0; i < 2000; i++) {
    const stick: [number, number] = [axis(), axis()];
    const [x, y] = stickDot(stick);
    const length = Math.hypot(x, y);
    expect(length).toBeLessThanOrEqual(1 + 1e-12);
    expect([Math.sign(x), Math.sign(y)]).toEqual([Math.sign(stick[0]), Math.sign(stick[1])]);
    const raw = Math.hypot(Math.max(-1, stick[0] / 32767), Math.max(-1, stick[1] / 32767));
    if (raw >= 1) expect(length).toBeCloseTo(1, 12);
    else expect(length).toBeCloseTo(raw, 12);
  }
});

test("every binding choice round-trips through its dropdown value, and the dropdown's labels are distinct", () => {
  const presses: Press[] = ["left_click", "right_click", "pointer", ...KEYS.map((key) => ({ key }))];
  for (const press of presses) expect(pressFromValue(pressValue(press))).toEqual(press);
  const options = pressOptions();
  expect(options.map(([value]) => value)).toEqual(presses.map(pressValue));
  expect(new Set(options.map(([, label]) => label)).size).toBe(options.length);
  for (const key of KEYS.filter((k) => k.length === 1)) expect(pressLabel({ key })).toBe(key.toUpperCase());
});

test("a hash names its page; any other hash opens the first page (seed 4, 500 cases)", () => {
  const next = random(4);
  const page = () => () => {};
  const paths = ["/", "/controller", "/history", "/stats", "/nope", "", "/history/x"];
  for (let i = 0; i < 500; i++) {
    const routes: Route[] = paths.filter(() => next() < 0.5).map((path) => ({ path, title: `t${path}`, page }));
    if (routes.length === 0) continue;
    const hash = `${next() < 0.7 ? "#" : ""}${paths[Math.floor(next() * paths.length)]}`;
    const path = hash.replace(/^#/, "") || "/";
    expect(resolve(routes, hash)).toBe(routes.find((r) => r.path === path) ?? routes[0]!);
  }
});

test("a stick's four directions collapse to one line exactly when they match, and its click never repeats the stick's name (seed 4, 2000 cases)", () => {
  const next = random(4);
  const ARROWS = ["↑", "↓", "←", "→"];
  const DIRECTIONS = ["up", "down", "left", "right"];
  const pick = () => [undefined, "Jump", "Attack"][Math.floor(next() * 3)];
  for (let i = 0; i < 2000; i++) {
    const shared = pick();
    const actions = DIRECTIONS.map(() => (next() < 0.5 ? shared : pick()));
    const click = pick();
    const fight: Binding[] = [
      ...actions.flatMap((action, n) => (action === undefined ? [] : [{ control: `left_${DIRECTIONS[n]}` as Control, action, press: "pointer" as const }])),
      ...(click ? [{ control: "left_stick" as const, action: click, press: "pointer" as const }] : []),
    ];
    const lines = callouts(fight, []).get("left")!;
    expect(lines[0]).toEqual({ tag: "Left stick", text: "", kind: "fight" });
    const moves = lines.filter((l, n) => n > 0 && !l.tag.startsWith("Click"));
    const clicks = lines.filter((l) => l.tag.startsWith("Click"));
    if (actions.every((a) => a === undefined)) expect(moves).toEqual([]);
    else if (actions.every((a) => a === actions[0])) expect(moves).toEqual([{ tag: "", text: actions[0]!, kind: "fight" }]);
    else expect(moves).toEqual(actions.map((a, n) => ({ tag: ARROWS[n]!, text: a ?? "Not used", kind: a ? "fight" : "none" })));
    expect(clicks).toEqual([{ tag: "Click", text: click ?? "Not used", kind: click ? "fight" : "none" }]);
  }
});

test("a button shows its fight action, a different menu action under Menus, and Not used when it has neither; an empty D-pad reads Not used", () => {
  const bind = (control: Control, action: string): Binding => ({ control, action, press: "pointer" });
  const lines = callouts([bind("a", "Attack"), bind("b", "Shield")], [bind("a", "Confirm"), bind("b", "Shield"), bind("x", "Back"), bind("left_stick", "Open")]);
  expect(lines.get("a")).toEqual([{ tag: "A", text: "Attack", kind: "fight" }, { tag: "Menus", text: "Confirm", kind: "menus" }]);
  expect(lines.get("b")).toEqual([{ tag: "B", text: "Shield", kind: "fight" }]);
  expect(lines.get("x")).toEqual([{ tag: "X · Menus", text: "Back", kind: "menus" }]);
  expect(lines.get("y")).toEqual([{ tag: "Y", text: "Not used", kind: "none" }]);
  expect(lines.get("left")!.at(-1)).toEqual({ tag: "Click · Menus", text: "Open", kind: "menus" });
  expect(lines.get("dpad")).toEqual([{ tag: "D-pad", text: "Not used", kind: "none" }]);
});
