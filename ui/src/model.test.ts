import { expect, test } from "bun:test";
import { pressed, pressFromValue, pressLabel, pressValue, stickDot, type InputView } from "./model";
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
