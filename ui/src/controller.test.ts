import { expect, test } from "bun:test";
import { KEYS, pressed, pressFromValue, pressLabel, pressOptions, pressValue, stickDot, type InputView, type Press } from "./controller";
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
