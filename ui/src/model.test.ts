import { expect, test } from "bun:test";
import { pressed, pressFromValue, pressLabel, pressValue, stickDot, smashcraftActionLabel, type InputView } from "./model";
import { resolve, type Route } from "./router";

test("button bits follow the service's order", () => {
  const input: InputView = { left: [0, 0], right: [0, 0], lt: 0, rt: 0, buttons: 1 | (1 << 6) };
  expect(pressed(input, "a")).toBe(true);
  expect(pressed(input, "start")).toBe(true);
  expect(pressed(input, "b")).toBe(false);
});

test("sticks stay inside their circle and point up when pushed up", () => {
  expect(stickDot([0, -32768])).toEqual([0, -1]);
  const [x, y] = stickDot([32767, 32767]);
  expect(Math.hypot(x, y)).toBeCloseTo(1);
});

test("binding choices round-trip and read as players know them", () => {
  for (const press of [{ key: "q" }, { key: "f10" }, "left_click", "pointer"] as const) {
    expect(pressFromValue(pressValue(press))).toEqual(press);
  }
  expect(pressLabel({ key: "escape" })).toBe("Esc");
  expect(pressLabel({ key: "q" })).toBe("Q");
});

test("unknown routes open the first page", () => {
  const page = () => () => {};
  const routes: Route[] = [{ path: "/controller", title: "Controller", page }, { path: "/history", title: "History", page }];
  expect(resolve(routes, "#/history").title).toBe("History");
  expect(resolve(routes, "#/nope").title).toBe("Controller");
  expect(resolve(routes, "").title).toBe("Controller");
});

test("Smashcraft labels the LB modifier Tilt", () => {
  expect(smashcraftActionLabel({ control: "lb", action: "Walk", press: { key: "p" } })).toBe("Tilt");
  expect(smashcraftActionLabel({ control: "a", action: "Attack", press: { key: "5" } })).toBe("Attack");
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
      pad_preset: "standard", tap_jump: false, triggers: { left: "full", right: "full" }, pad_presets: [
        { preset: "standard", label: "Standard", bindings: bindings(false) },
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
    expect(select.value).toBe("standard");
    select.value = "z-jump";
    await select.handlers.get("change")!();
    expect(selected).toBe("z-jump");
    const publish = (preset: string, tap: boolean, left: string, right: string) => onState?.({ link: "connected", wanted: true, view: { rows: [], problem: null }, snapshot: { profile: "smashcraft", settings: { pad_preset: preset, tap_jump: tap, triggers: { left, right } } } } as unknown as import("./model").ControllerState);
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
    publish("standard", true, "full", "light");
    expect(select.value).toBe("standard");
    expect(toggle.checked).toBe(true);
    expect(selects[1]!.value).toBe("full");
    expect(right.value).toBe("light");
    stop();
  } finally {
    Object.assign(api, apiBefore);
    globalThis.document = documentBefore;
  }
});
