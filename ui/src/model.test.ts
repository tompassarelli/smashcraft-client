import { expect, test } from "bun:test";
import { KEYS, pressed, pressFromValue, pressLabel, pressOptions, pressValue, stickDot, smashcraftActionLabel, type Control, type InputView, type Press } from "./model";
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

test("in Smashcraft every binding reads as its action, except LB, the Tilt modifier (seed 5, 500 cases)", () => {
  const next = random(5);
  const controls = Object.keys({ a: 0, b: 0, x: 0, y: 0, lb: 0, rb: 0, lt: 0, rt: 0, start: 0, back: 0, left_stick: 0, dpad_up: 0, left_up: 0, right_down: 0 }) as Control[];
  for (let i = 0; i < 500; i++) {
    const control = controls[Math.floor(next() * controls.length)]!;
    const action = ["Walk", "Attack", "Jump", "Tilt", "Grab", ""][Math.floor(next() * 6)]!;
    expect(smashcraftActionLabel({ control, action, press: { key: "p" } })).toBe(control === "lb" ? "Tilt" : action);
  }
});

test("the Controller page changes choices and follows service snapshots", async () => {
  const { api } = await import("./tauri");
  const { controllerPage } = await import("./pages/controller");
  class Element {
    children: (Element | string)[] = [];
    handlers = new Map<string, () => unknown>();
    className = "";
    textContent = "";
    value = "";
    checked = false;
    type = "";
    classList = { add: () => {}, toggle: () => {} };
    constructor(readonly tag: string) {}
    append(...children: (Element | string)[]) { this.children.push(...children); }
    replaceChildren(...children: (Element | string)[]) { this.children = children; }
    addEventListener(event: string, handler: () => unknown) { this.handlers.set(event, handler); }
    setAttribute() {}
  }
  const documentBefore = globalThis.document;
  const apiBefore = { ...api };
  const elements: Element[] = [];
  const createElement = (tag: string) => { const el = new Element(tag); elements.push(el); return el; };
  const bindings = (zJump: boolean) => [
    ["a", "Attack"], ["x", "Special"], ["b", zJump ? "Grab" : "Jump"],
    ["rb", zJump ? "Jump" : "Grab"], ["y", "Jump"], ["lb", "Tilt"],
    ["lt", "Shield"], ["rt", "Shield"], ["left_up", "Up"],
  ].map(([control, action]) => ({ control, action, press: { key: "i" } })) as import("./model").Binding[];
  let selected: string | undefined;
  let tapJump: boolean | undefined;
  let triggers: import("./model").TriggerShields | undefined;
  let onState: ((state: import("./model").ControllerState) => void) | undefined;
  globalThis.document = { createElement, createElementNS: (_namespace: string, tag: string) => createElement(tag) } as unknown as Document;
  Object.assign(api, {
    controllerState: async () => ({ link: "off", wanted: false, view: { rows: [], problem: null }, snapshot: {} }),
    startWithComputer: async () => false,
    onController: async (handler: typeof onState) => { onState = handler; return () => {}; },
    onInput: async () => () => {},
    bindings: async () => ({
      smashcraft: bindings(false), smashcraft_menus: [], any_map: [], any_map_defaults: [], profile: "auto",
      pad_preset: "melee", tap_jump: false, triggers: { left: "full", right: "full" }, pad_presets: [
        { preset: "melee", label: "Melee", bindings: bindings(false) },
        { preset: "z-jump", label: "Z-jump", bindings: bindings(true) },
      ],
    }),
    setPadPreset: async (preset: string) => { selected = preset; return true; },
    setTapJump: async (on: boolean) => { tapJump = on; return true; },
    setTriggerShields: async (choice: import("./model").TriggerShields) => { triggers = choice; return true; },
  });
  try {
    const stop = controllerPage(new Element("div") as unknown as HTMLElement);
    await Promise.resolve();
    const select = elements.find((el) => el.tag === "select")!;
    expect(select.value).toBe("melee");
    select.value = "z-jump";
    await select.handlers.get("change")!();
    expect(selected).toBe("z-jump");
    const publish = (preset: string, tap: boolean, left: string, right: string) => onState?.({ link: "connected", wanted: true, view: { rows: [], problem: null }, snapshot: { profile: "map", settings: { pad_preset: preset, tap_jump: tap, triggers: { left, right } } } } as unknown as import("./model").ControllerState);
    publish("z-jump", false, "light", "full");
    const table = elements.find((el) => el.tag === "table")!;
    const rows = table.children.map((row) => (row as Element).children.map((cell) => (cell as Element).children[0]));
    expect(rows).toEqual([
      ["A", "Attack"], ["X", "Special"], ["B", "Grab"], ["RB", "Jump"], ["Y", "Jump"],
      ["LB", "Tilt"], ["LT", "Light shield"], ["RT", "Shield"], ["Left stick up", "Up"],
    ]);
    const toggle = elements.find((el) => el.tag === "input" && el.type === "checkbox")!;
    expect(toggle.checked).toBe(false);
    toggle.checked = true;
    await toggle.handlers.get("change")!();
    expect(tapJump).toBe(true);
    const selects = elements.filter((el) => el.tag === "select");
    const right = selects[2]!;
    right.value = "light";
    await right.handlers.get("change")!();
    expect(triggers).toEqual({ left: "light", right: "light" });
    publish("melee", true, "full", "light");
    expect(select.value).toBe("melee");
    expect(toggle.checked).toBe(true);
    expect(selects[1]!.value).toBe("full");
    expect(right.value).toBe("light");
    stop();
  } finally {
    Object.assign(api, apiBefore);
    globalThis.document = documentBefore;
  }
});
