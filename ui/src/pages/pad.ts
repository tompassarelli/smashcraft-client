import { h, svg } from "../dom";
import { callouts, type Line } from "../bindings";
import { pressed, stickDot, type Binding, type Button, type InputView } from "../controller";

export const NEUTRAL: InputView = { left: [0, 0], right: [0, 0], lt: 0, rt: 0, buttons: 0 };

const S = 1.2;
const PAD = { x: 200, y: 0 };
const at = (x: number, y: number): [number, number] => [PAD.x + x * S, PAD.y + y * S];
const LINE = 19;
const GAP = 10;
const LEFT_X = PAD.x - 22;
const RIGHT_X = PAD.x + 360 * S + 22;

const ANCHORS: { part: string; side: "left" | "right"; point: [number, number]; via?: [number, number] }[] = [
  { part: "lt", side: "left", point: at(80, 8) },
  { part: "lb", side: "left", point: at(55, 27) },
  { part: "left", side: "left", point: at(57, 95) },
  { part: "dpad", side: "left", point: at(98, 135) },
  { part: "rt", side: "right", point: at(280, 8) },
  { part: "rb", side: "right", point: at(305, 27) },
  { part: "y", side: "right", point: at(296, 62) },
  { part: "x", side: "right", point: at(262, 85), via: at(296, 79) },
  { part: "b", side: "right", point: at(319, 85) },
  { part: "a", side: "right", point: at(296, 108) },
  { part: "right", side: "right", point: at(261, 140) },
];
const BELOW: { part: string; point: [number, number]; anchor: "end" | "start" }[] = [
  { part: "back", point: at(160, 82), anchor: "end" },
  { part: "start", point: at(200, 82), anchor: "start" },
];

function calloutLabels(root: SVGElement, leaders: SVGElement, labels: SVGElement): (fight: Binding[], menus: Binding[]) => void {
  const text = (x: number, y: number, anchor: "start" | "end", line: Line) => {
    const t = svg("text", { x, y, class: `callout ${anchor} ${line.kind}` });
    const tag = svg("tspan", { class: line.text ? "tag" : "title" });
    tag.textContent = line.text ? `${line.tag} ` : line.tag;
    t.append(tag);
    if (line.text) {
      const action = svg("tspan", { class: "action" });
      action.textContent = line.text;
      t.append(action);
    }
    labels.append(t);
  };
  const label = (fight: Binding[], menus: Binding[]) => {
    const lines = callouts(fight, menus);
    leaders.replaceChildren();
    labels.replaceChildren();
    let bottom = 0;
    for (const side of ["left", "right"] as const) {
      let next = -Infinity;
      for (const { part, point, via } of ANCHORS.filter((a) => a.side === side)) {
        const block = lines.get(part) ?? [];
        const first = Math.max(point[1] + 5 - ((block.length - 1) * LINE) / 2, next);
        const x = side === "left" ? LEFT_X : RIGHT_X;
        block.forEach((line, i) => text(x, first + i * LINE, side === "left" ? "end" : "start", line));
        const y = first - 5;
        const elbow = side === "left" ? x + 8 : x - 8;
        const points = [point, ...(via ? [via] : []), [elbow, y], [side === "left" ? x + 4 : x - 4, y]];
        leaders.append(svg("polyline", { points: points.map((p) => p.join(",")).join(" ") }));
        next = first + block.length * LINE + GAP;
        bottom = Math.max(bottom, next - GAP);
      }
    }
    const below = PAD.y + 205 * S + 18;
    for (const { part, point, anchor } of BELOW) {
      const block = lines.get(part) ?? [];
      const x = point[0] + (anchor === "end" ? 6 : -6);
      block.forEach((line, i) => text(x, below + i * LINE, anchor, line));
      leaders.append(svg("polyline", { points: `${point[0]},${point[1]} ${point[0]},${below - 14}` }));
      bottom = Math.max(bottom, below + block.length * LINE);
    }
    root.setAttribute("viewBox", `0 -12 ${RIGHT_X + 190} ${bottom + 12}`);
  };
  return label;
}

export function padView(): { el: HTMLElement; render: (input: InputView) => void; label: (fight: Binding[], menus: Binding[]) => void } {
  const root = svg("svg", { class: "pad", role: "img", "aria-label": "Your controller, each button labelled with what it does" });
  const leaders = svg("g", { class: "leaders" });
  const labels = svg("g", { class: "callouts" });
  const pad = svg("g", { transform: `translate(${PAD.x} ${PAD.y}) scale(${S})` });
  pad.append(svg("path", {
    d: "M70 40 Q180 20 290 40 Q345 50 350 120 Q355 190 310 195 Q280 198 255 160 L105 160 Q80 198 50 195 Q5 190 10 120 Q15 50 70 40Z",
    class: "body",
  }));
  root.append(pad, leaders);
  const controls = svg("g", { transform: `translate(${PAD.x} ${PAD.y}) scale(${S})` });
  root.append(controls, labels);
  const trigger = (x: number, label: string) => {
    controls.append(svg("rect", { x, y: 2, width: 60, height: 12, rx: 4, class: "trigger-bg" }));
    const fill = svg("rect", { x, y: 2, width: 0, height: 12, rx: 4, class: "trigger-fill" });
    const text = svg("text", { x: x + 30, y: 11.5, class: "tiny" });
    text.textContent = label;
    controls.append(fill, text);
    return fill;
  };
  const lt = trigger(50, "LT");
  const rt = trigger(250, "RT");
  const buttons = new Map<Button, SVGElement>();
  const button = (name: Button, shape: SVGElement, label?: [number, number, string]) => {
    shape.classList.add("btn");
    controls.append(shape);
    if (label) {
      const text = svg("text", { x: label[0], y: label[1], class: "label" });
      text.textContent = label[2];
      controls.append(text);
    }
    buttons.set(name, shape);
  };
  button("lb", svg("rect", { x: 50, y: 20, width: 60, height: 14, rx: 6 }), [80, 31, "LB"]);
  button("rb", svg("rect", { x: 250, y: 20, width: 60, height: 14, rx: 6 }), [280, 31, "RB"]);
  button("back", svg("rect", { x: 150, y: 72, width: 20, height: 10, rx: 5 }));
  button("start", svg("rect", { x: 190, y: 72, width: 20, height: 10, rx: 5 }));
  const face: [Button, number, number, string][] = [
    ["y", 285, 62, "Y"], ["x", 262, 85, "X"], ["b", 308, 85, "B"], ["a", 285, 108, "A"],
  ];
  for (const [name, cx, cy, text] of face) button(name, svg("circle", { cx, cy, r: 11, class: `face-${name}` }), [cx, cy + 4, text]);
  const dpad: [Button, number, number][] = [["dpad_up", 120, 120], ["dpad_down", 120, 150], ["dpad_left", 105, 135], ["dpad_right", 135, 135]];
  for (const [name, x, y] of dpad) button(name, svg("rect", { x: x - 7, y: y - 7, width: 14, height: 14, rx: 2 }));
  const stick = (cx: number, cy: number, click: Button) => {
    controls.append(svg("circle", { cx, cy, r: 26, class: "stick-well" }));
    const knob = svg("circle", { cx, cy, r: 13, class: "stick" });
    controls.append(knob);
    buttons.set(click, knob);
    return (value: [number, number]) => {
      const [dx, dy] = stickDot(value);
      knob.setAttribute("cx", String(cx + dx * 16));
      knob.setAttribute("cy", String(cy + dy * 16));
      knob.classList.toggle("moved", Math.hypot(dx, dy) > 0.28);
    };
  };
  const left = stick(75, 95, "left_stick");
  const right = stick(235, 140, "right_stick");
  const render = (input: InputView) => {
    lt.setAttribute("width", String((60 * input.lt) / 32767));
    rt.setAttribute("width", String((60 * input.rt) / 32767));
    for (const [name, el] of buttons) el.classList.toggle("on", pressed(input, name));
    left(input.left);
    right(input.right);
  };
  const label = calloutLabels(root, leaders, labels);
  render(NEUTRAL);
  label([], []);
  const hint = h("p.note", {}, "Press buttons and move the sticks: they light up here.");
  return { el: h("div.pad-view", {}, root as unknown as HTMLElement, hint), render, label };
}
