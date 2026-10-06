// Mirrors smashcraft:companion/model (Rust, serde) and the client's commands.

export type Light = "green" | "amber" | "red" | "off";
export type Link = "off" | "starting" | "connected";
export type Profile = "smashcraft" | "any_map" | "off";
export type ProfileChoice = "auto" | Profile;

export type Row = { label: string; text: string; light: Light };
export type View = { rows: [Row, Row, Row]; overall: Light; problem: string | null };

export type Snapshot = {
  pad: { name: string; id: string } | null;
  game: { pid: number; window: boolean } | null;
  session: { map: string; phase: string; player: number | null } | null;
  profile: Profile;
  choice: ProfileChoice;
  output: { running: boolean; ready: boolean; focused: boolean };
  problem: string | null;
};

export type ControllerState = { link: Link; wanted: boolean; view: View; snapshot: Snapshot };

/** SDL axes: -32768..32767, y negative when pushed up. Triggers 0..32767. */
export type InputView = { left: [number, number]; right: [number, number]; lt: number; rt: number; buttons: number };

export const BUTTONS = [
  "a", "b", "x", "y", "lb", "rb", "start", "back", "left_stick", "right_stick",
  "dpad_up", "dpad_down", "dpad_left", "dpad_right",
] as const;
export type Button = (typeof BUTTONS)[number];

export const pressed = (input: InputView, button: Button): boolean =>
  (input.buttons & (1 << BUTTONS.indexOf(button))) !== 0;

export type Control =
  | "a" | "b" | "x" | "y" | "lb" | "rb" | "lt" | "rt" | "start" | "back"
  | "dpad_up" | "dpad_down" | "dpad_left" | "dpad_right"
  | "left_up" | "left_down" | "left_left" | "left_right"
  | "right_up" | "right_down" | "right_left" | "right_right";

export type Press = { key: string } | "left_click" | "right_click" | "pointer";
export type Binding = { control: Control; action: string; press: Press };
export type Bindings = { smashcraft: Binding[]; smashcraft_menus: Binding[]; any_map: Binding[]; any_map_defaults: Binding[]; profile: ProfileChoice };

export type PlayState = { available: boolean; running: boolean; lines: string[]; finished: boolean | null };

export const PROFILE_CHOICES: { choice: ProfileChoice; label: string; hint: string }[] = [
  { choice: "auto", label: "Automatic", hint: "Smashcraft controls in Smashcraft, your own keys everywhere else" },
  { choice: "smashcraft", label: "Smashcraft", hint: "Smashcraft's fighting controls and menus" },
  { choice: "any_map", label: "Any map", hint: "Your own keys and mouse for any map" },
  { choice: "off", label: "Off", hint: "Your controller doesn't press anything" },
];

export const PROFILE_LABEL: Record<Profile, string> = { smashcraft: "Smashcraft", any_map: "Any map", off: "Off" };

const CONTROL_LABEL: Record<Control, string> = {
  a: "A", b: "B", x: "X", y: "Y", lb: "LB", rb: "RB", lt: "LT", rt: "RT", start: "Start", back: "Back",
  dpad_up: "D-pad up", dpad_down: "D-pad down", dpad_left: "D-pad left", dpad_right: "D-pad right",
  left_up: "Left stick up", left_down: "Left stick down", left_left: "Left stick left", left_right: "Left stick right",
  right_up: "Right stick up", right_down: "Right stick down", right_left: "Right stick left", right_right: "Right stick right",
};

export const controlLabel = (control: Control): string => CONTROL_LABEL[control];

const KEY_LABEL: Record<string, string> = {
  space: "Space", escape: "Esc", tab: "Tab", enter: "Enter", up: "↑", down: "↓", left: "←", right: "→",
};

export const KEYS: string[] = [
  ..."abcdefghijklmnopqrstuvwxyz0123456789".split(""),
  "space", "escape", "tab", "enter", "up", "down", "left", "right",
  ...Array.from({ length: 12 }, (_, i) => `f${i + 1}`),
];

export function pressLabel(press: Press): string {
  if (press === "left_click") return "Left click";
  if (press === "right_click") return "Right click";
  if (press === "pointer") return "Mouse pointer";
  return KEY_LABEL[press.key] ?? press.key.toUpperCase();
}

/** Every choice a binding's dropdown offers, as [value, label]. */
export function pressOptions(): [string, string][] {
  return [
    ["left_click", "Left click"],
    ["right_click", "Right click"],
    ["pointer", "Mouse pointer"],
    ...KEYS.map((key): [string, string] => [`key:${key}`, pressLabel({ key })]),
  ];
}

export const pressValue = (press: Press): string => (typeof press === "string" ? press : `key:${press.key}`);

export function pressFromValue(value: string): Press {
  if (value === "left_click" || value === "right_click" || value === "pointer") return value;
  return { key: value.replace(/^key:/, "") };
}

/** A stick's dot position inside a unit circle, y down as on screen. */
export function stickDot([x, y]: [number, number]): [number, number] {
  const fx = Math.max(-1, Math.min(1, x / 32767));
  const fy = Math.max(-1, Math.min(1, y / 32767));
  const length = Math.hypot(fx, fy);
  return length > 1 ? [fx / length, fy / length] : [fx, fy];
}
