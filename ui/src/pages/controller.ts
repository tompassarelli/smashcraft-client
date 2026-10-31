import { h } from "../dom";
import { PROFILE_CHOICES, PROFILE_LABEL, type Bindings, type ControllerState, type ControllerSettings, type ProfileChoice, type RightStick } from "../controller";
import type { Api } from "../api";
import { NEUTRAL, padView } from "./pad";
import { statusCard } from "./status";

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

function mappingCard(pad: ReturnType<typeof padView>, api: Api) {
  const presetSelect = h("select", { "aria-label": "Smashcraft pad preset" });
  const triggerSelects = { left: h("select", { "aria-label": "Left trigger shield" }), right: h("select", { "aria-label": "Right trigger shield" }) };
  for (const select of Object.values(triggerSelects)) {
    select.append(h("option", { value: "full" }, "Full"), h("option", { value: "light" }, "Light"));
    select.addEventListener("change", async () => {
      select.disabled = true;
      try {
        await api.setTriggerShields({ left: triggerSelects.left.value as "full" | "light", right: triggerSelects.right.value as "full" | "light" });
      } finally { select.disabled = false; }
    });
  }
  const rightStick = h("select", { "aria-label": "Right stick" }, h("option", { value: "smash" }, "Smash attacks"), h("option", { value: "tilt" }, "Tilt attacks"));
  rightStick.addEventListener("change", async () => {
    rightStick.disabled = true;
    try {
      await api.setRightStick(rightStick.value as RightStick);
    } finally {
      rightStick.disabled = false;
    }
  });
  const tapJump = h("input", { type: "checkbox", "aria-label": "Tap jump" });
  tapJump.addEventListener("change", async () => {
    tapJump.disabled = true;
    try {
      await api.setTapJump(tapJump.checked);
    } finally {
      tapJump.disabled = false;
    }
  });
  let presetBindings: Bindings["pad_presets"] = [];
  presetSelect.addEventListener("change", async () => {
    const preset = presetBindings.find((p) => p.preset === presetSelect.value);
    if (!preset) return;
    presetSelect.disabled = true;
    try {
      await api.setPadPreset(preset.preset);
    } finally {
      presetSelect.disabled = false;
    }
  });
  const label = (bindings: Bindings) => pad.label(bindings.smashcraft, bindings.smashcraft_menus);
  let currentSettings: ControllerSettings | undefined;
  const renderSettings = (settings: ControllerSettings) => {
    currentSettings = settings;
    tapJump.checked = settings.tap_jump;
    presetSelect.value = settings.pad_preset;
    triggerSelects.left.value = settings.triggers.left;
    triggerSelects.right.value = settings.triggers.right;
    rightStick.value = settings.right_stick;
    void api.bindings().then(label);
  };
  const load = (bindings: Bindings) => {
    presetBindings = bindings.pad_presets;
    presetSelect.replaceChildren(...presetBindings.map((p) => h("option", { value: p.preset }, p.label)));
    label(bindings);
    renderSettings(currentSettings ?? bindings);
  };
  const settings = h(
    "div.pad-settings",
    {},
    h("label.setting", {}, h("span", {}, "Layout"), presetSelect),
    h("label.setting.switch", {}, tapJump, h("span", {}, "Tap jump"), h("small", {}, "Push the left stick up to jump.")),
    h("div.setting", {}, h("span", {}, "Trigger shields"),
      h("div.shields", {}, h("label", {}, "LT ", triggerSelects.left), h("label", {}, "RT ", triggerSelects.right))),
    h("label.setting", {}, h("span", {}, "Right stick"), rightStick),
  );
  const el = h("section.card.mapping-card", {}, h("h2", {}, "Smashcraft buttons"), h("div.pad-layout", {}, pad.el, settings));
  return { el, renderSettings, load };
}

function settingsCard(api: Api) {
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
    h("h2", {}, "App"),
    h("label.switch", {}, toggle, h("span", {}, "Start with my computer"), h("small", {}, "Smashcraft waits in the tray, ready for your controller.")),
  );
}

export function controllerPage(root: HTMLElement, api: Api): () => void {
  const status = statusCard(api);
  const pad = padView();
  const profile = profileCard((choice) => void api.setProfile(choice));
  const mapping = mappingCard(pad, api);
  root.append(
    status.el,
    mapping.el,
    h("div.grid", {}, profile.el, settingsCard(api)),
  );
  let link: ControllerState["link"] = "off";
  const onState = (s: ControllerState) => {
    link = s.link;
    status.render(s);
    profile.render(s);
    if (s.link === "connected") mapping.renderSettings(s.snapshot.settings);
    if (s.link !== "connected") pad.render(NEUTRAL);
  };
  void api.controllerState().then(onState);
  void api.playState().then((state) => status.setMap(state.map));
  void api.bindings().then((b) => {
    profile.select(b.profile);
    mapping.load(b);
  });
  const unlisten = [api.onController(onState), api.onPlay((state) => status.setMap(state.map)), api.onInput((input) => link === "connected" && pad.render(input))];
  return () => unlisten.forEach((p) => void p.then((stop) => stop()));
}
