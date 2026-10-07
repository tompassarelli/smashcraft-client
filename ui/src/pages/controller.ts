import { h, svg } from "../dom";
import {
  PROFILE_CHOICES, PROFILE_LABEL, controlLabel, pressFromValue, pressLabel, pressOptions, pressValue, pressed, stickDot, smashcraftActionLabel,
  type Binding, type Bindings, type Button, type ControllerState, type InputView, type ProfileChoice,
} from "../model";
import { api } from "../tauri";

const NEUTRAL: InputView = { left: [0, 0], right: [0, 0], lt: 0, rt: 0, buttons: 0 };

function statusCard(): { el: HTMLElement; render: (s: ControllerState) => void } {
  const rows = h("div.rows");
  const problem = h("p.problem", { hidden: true });
  const turnOn = h("button.primary", { type: "button" }, "Turn on controller support");
  const offNote = h(
    "p.note",
    {},
    "Controller support lets your gamepad play Warcraft III. It keeps running in the background, also when this window is closed.",
  );
  const off = h("div.turn-on", { hidden: true }, offNote, turnOn);
  turnOn.addEventListener("click", async () => {
    turnOn.disabled = true;
    try {
      render(await api.turnOnController());
    } finally {
      turnOn.disabled = false;
    }
  });
  const render = (s: ControllerState) => {
    rows.replaceChildren(
      ...s.view.rows.map((row) =>
        h(`div.row.${row.light}`, {}, h("span.light"), h("span.label", {}, row.label), h("span.text", {}, row.text)),
      ),
    );
    problem.hidden = !s.view.problem;
    problem.textContent = s.view.problem ?? "";
    off.hidden = s.link !== "off";
    turnOn.textContent = s.wanted ? "Try again" : "Turn on controller support";
  };
  return { el: h("section.card.status", {}, h("h2", {}, "Status"), rows, problem, off), render };
}

function padView(): { el: HTMLElement; render: (input: InputView) => void } {
  const root = svg("svg", { viewBox: "0 0 360 210", class: "pad", role: "img", "aria-label": "Your controller's buttons and sticks" });
  const body = svg("path", {
    d: "M70 40 Q180 20 290 40 Q345 50 350 120 Q355 190 310 195 Q280 198 255 160 L105 160 Q80 198 50 195 Q5 190 10 120 Q15 50 70 40Z",
    class: "body",
  });
  root.append(body);
  const trigger = (x: number, label: string) => {
    const g = svg("g");
    g.append(svg("rect", { x, y: 2, width: 60, height: 12, rx: 4, class: "trigger-bg" }));
    const fill = svg("rect", { x, y: 2, width: 0, height: 12, rx: 4, class: "trigger-fill" });
    const text = svg("text", { x: x + 30, y: 11.5, class: "tiny" });
    text.textContent = label;
    g.append(fill, text);
    root.append(g);
    return fill;
  };
  const lt = trigger(50, "LT");
  const rt = trigger(250, "RT");
  const buttons = new Map<Button, SVGElement>();
  const button = (name: Button, shape: SVGElement, label?: [number, number, string]) => {
    shape.classList.add("btn");
    root.append(shape);
    if (label) {
      const text = svg("text", { x: label[0], y: label[1], class: "label" });
      text.textContent = label[2];
      root.append(text);
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
    root.append(svg("circle", { cx, cy, r: 26, class: "stick-well" }));
    const knob = svg("circle", { cx, cy, r: 13, class: "stick" });
    root.append(knob);
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
  render(NEUTRAL);
  const hint = h("p.note", {}, "Press buttons and move the sticks: they light up here.");
  return { el: h("section.card.input", {}, h("h2", {}, "Your controller"), root as unknown as HTMLElement, hint), render };
}

function profileCard(onChoice: (choice: ProfileChoice) => void) {
  const running = h("p.note");
  const options = PROFILE_CHOICES.map(({ choice, label, hint }) => {
    const input = h("input", { type: "radio", name: "profile", value: choice });
    input.addEventListener("change", () => onChoice(choice));
    return { choice, input, el: h("label.choice", { title: hint }, input, h("span", {}, label), h("small", {}, hint)) };
  });
  const select = (choice: ProfileChoice) => options.forEach((o) => (o.input.checked = o.choice === choice));
  const render = (s: ControllerState) => {
    running.textContent = s.link === "connected" ? `Using now: ${PROFILE_LABEL[s.snapshot.profile]}` : "";
  };
  return {
    el: h("section.card.profile", {}, h("h2", {}, "Controls"), h("div.choices", {}, ...options.map((o) => o.el)), running),
    select,
    render,
  };
}

function mappingCard() {
  const smashcraftTable = h("table.mapping");
  const presetSelect = h("select", { "aria-label": "Smashcraft pad preset" });
  let presetBindings: Bindings["pad_presets"] = [];
  presetSelect.addEventListener("change", async () => {
    const preset = presetBindings.find((p) => p.preset === presetSelect.value);
    if (!preset) return;
    presetSelect.disabled = true;
    try {
      await api.setPadPreset(preset.preset);
      smashcraftTable.replaceChildren(...rows(preset.bindings, true));
    } finally {
      presetSelect.disabled = false;
    }
  });
  const menusTable = h("table.mapping");
  const anyMapTable = h("table.mapping");
  const reset = h("button.quiet", { type: "button" }, "Back to the default keys");
  let anyMap: Binding[] = [];
  const save = async () => {
    await api.setAnyMapBindings(anyMap);
  };
  const rows = (bindings: Binding[], smashcraft = false) => {
    // A stick's four directions doing the same thing read as one row.
    const seen = new Set<string>();
    return bindings.flatMap((b) => {
      const stick = /^(left|right)_/.exec(b.control)?.[1];
      const label = stick !== undefined && b.press === "pointer" ? `${stick === "left" ? "Left" : "Right"} stick` : controlLabel(b.control);
      if (seen.has(label)) return [];
      seen.add(label);
      return [h("tr", {}, h("th", {}, label), h("td", {}, smashcraft ? smashcraftActionLabel(b) : b.action))];
    });
  };
  const renderSmashcraft = (bindings: Binding[], menus: Binding[]) => {
    smashcraftTable.replaceChildren(...rows(bindings, true));
    menusTable.replaceChildren(...rows(menus));
  };
  const renderAnyMap = () => {
    const options = pressOptions();
    anyMapTable.replaceChildren(
      ...anyMap.map((b, i) => {
        const select = h("select", { "aria-label": `${controlLabel(b.control)} presses` });
        for (const [value, label] of options) select.append(h("option", { value }, label));
        select.value = pressValue(b.press);
        select.addEventListener("change", () => {
          anyMap[i] = { ...b, press: pressFromValue(select.value), action: pressLabel(pressFromValue(select.value)) };
          void save();
        });
        return h("tr", {}, h("th", {}, controlLabel(b.control)), h("td", {}, select));
      }),
    );
  };
  const load = (bindings: Bindings, defaults?: Binding[]) => {
    presetBindings = bindings.pad_presets;
    presetSelect.replaceChildren(...presetBindings.map((p) => h("option", { value: p.preset }, p.label)));
    presetSelect.value = bindings.pad_preset;
    renderSmashcraft(bindings.smashcraft, bindings.smashcraft_menus);
    anyMap = (defaults ?? bindings.any_map).map((b) => ({ ...b }));
    renderAnyMap();
  };
  let defaults: Binding[] | undefined;
  reset.addEventListener("click", async () => {
    if (!defaults) return;
    anyMap = defaults.map((b) => ({ ...b }));
    renderAnyMap();
    await save();
  });
  const el = h(
    "section.card.mapping-card",
    {},
    h("h2", {}, "Buttons"),
    h("div.columns", {},
      h("div", {}, h("h3", {}, "Smashcraft"), h("label", {}, "Pad preset ", presetSelect), smashcraftTable, h("h3", {}, "Smashcraft menus"), menusTable),
      h("div", {}, h("h3", {}, "Any map"), anyMapTable, reset),
    ),
  );
  return {
    el,
    load: (bindings: Bindings, anyMapDefaults: Binding[]) => {
      defaults = anyMapDefaults;
      load(bindings);
    },
  };
}

function settingsCard() {
  const toggle = h("input", { type: "checkbox" });
  toggle.addEventListener("change", async () => {
    toggle.disabled = true;
    try {
      toggle.checked = await api.setStartWithComputer(toggle.checked);
    } finally {
      toggle.disabled = false;
    }
  });
  void api.startWithComputer().then((on) => (toggle.checked = on));
  return h(
    "section.card.settings",
    {},
    h("label.switch", {}, toggle, h("span", {}, "Start with my computer"), h("small", {}, "Smashcraft waits in the tray, ready for your controller.")),
  );
}

export function controllerPage(root: HTMLElement): () => void {
  const status = statusCard();
  const pad = padView();
  const profile = profileCard((choice) => void api.setProfile(choice));
  const mapping = mappingCard();
  root.append(
    h("div.grid", {}, status.el, pad.el, profile.el, settingsCard()),
    mapping.el,
  );
  let link: ControllerState["link"] = "off";
  const onState = (s: ControllerState) => {
    link = s.link;
    status.render(s);
    profile.render(s);
    if (s.link !== "connected") pad.render(NEUTRAL);
  };
  void api.controllerState().then(onState);
  void api.bindings().then((b) => {
    profile.select(b.profile);
    mapping.load(b, b.any_map_defaults);
  });
  const unlisten = [api.onController(onState), api.onInput((input) => link === "connected" && pad.render(input))];
  return () => unlisten.forEach((p) => void p.then((stop) => stop()));
}
