// The Rust side's commands and events (app.withGlobalTauri exposes window.__TAURI__).
import type { Bindings, Binding, ControllerState, InputView, PlayState, ProfileChoice } from "./model";

type TauriGlobal = {
  core: { invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> };
  event: { listen<T>(event: string, handler: (event: { payload: T }) => void): Promise<() => void> };
};

const tauri = (): TauriGlobal => (window as unknown as { __TAURI__: TauriGlobal }).__TAURI__;

const invoke = <T>(command: string, args?: Record<string, unknown>) => tauri().core.invoke<T>(command, args);

export const api = {
  controllerState: () => invoke<ControllerState>("controller_state"),
  turnOnController: () => invoke<ControllerState>("turn_on_controller"),
  setProfile: (choice: ProfileChoice) => invoke<boolean>("set_profile", { choice }),
  bindings: () => invoke<Bindings>("bindings"),
  setAnyMapBindings: (bindings: Binding[]) => invoke<boolean>("set_any_map_bindings", { bindings }),
  startWithComputer: () => invoke<boolean>("start_with_computer"),
  setStartWithComputer: (on: boolean) => invoke<boolean>("set_start_with_computer", { on }),
  playState: () => invoke<PlayState>("play_state"),
  play: () => invoke<PlayState>("play"),
  onController: (handler: (state: ControllerState) => void) =>
    tauri().event.listen<ControllerState>("controller", (e) => handler(e.payload)),
  onInput: (handler: (input: InputView) => void) => tauri().event.listen<InputView>("input", (e) => handler(e.payload)),
  onPlay: (handler: (state: PlayState) => void) => tauri().event.listen<PlayState>("play", (e) => handler(e.payload)),
};
