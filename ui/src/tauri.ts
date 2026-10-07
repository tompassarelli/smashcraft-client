// The Rust side's commands and events (app.withGlobalTauri exposes window.__TAURI__).
import type { Bindings, Binding, ControllerState, InputView, OnlineState, PlayState, ProfileChoice } from "./model";
import type { HistoryStore, RecordFile } from "./records";
import type { ReplayFile, WarcraftGame } from "./replays";

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
  recordFolders: () => invoke<string[]>("record_folders"),
  setRecordFolders: (folders: string[]) => invoke<string[]>("set_record_folders", { folders }),
  readRecords: () => invoke<RecordFile[]>("read_records"),
  history: () => invoke<unknown>("history"),
  saveHistory: (history: HistoryStore) => invoke<void>("save_history", { history }),
  onlineState: () => invoke<OnlineState>("online_state"),
  menuPageChoice: () => invoke<boolean | null>("menu_page_choice"),
  declineMenuPage: () => invoke<void>("decline_menu_page"),
  onlineSetup: () => invoke<OnlineState>("online_setup"),
  onlineHost: () => invoke<OnlineState>("online_host"),
  onlineJoin: (code: string) => invoke<OnlineState>("online_join", { code }),
  onlineStartNow: () => invoke<boolean>("online_start_now"),
  onlineCancel: () => invoke<void>("online_cancel"),
  onOnline: (handler: (state: OnlineState) => void) => tauri().event.listen<OnlineState>("online", (e) => handler(e.payload)),
  readReplays: () => invoke<ReplayFile[]>("read_replays"),
  readReplayParts: (folder: string, serial: number, parts: number) => invoke<string[]>("read_replay_parts", { folder, serial, parts }),
  keepReplay: (name: string, text: string) => invoke<string>("keep_replay", { name, text }),
  keepSim: (version: string, code: string) => invoke<void>("keep_sim", { version, code }),
  keptSim: (version: string) => invoke<string | null>("kept_sim", { version }),
  keptSims: () => invoke<string[]>("kept_sims"),
  warcraftGames: () => invoke<WarcraftGame[]>("warcraft_games"),
  watchInWarcraft: (file: string, name: string) => invoke<string>("watch_in_warcraft", { file, name }),
  onController: (handler: (state: ControllerState) => void) =>
    tauri().event.listen<ControllerState>("controller", (e) => handler(e.payload)),
  onInput: (handler: (input: InputView) => void) => tauri().event.listen<InputView>("input", (e) => handler(e.payload)),
  onPlay: (handler: (state: PlayState) => void) => tauri().event.listen<PlayState>("play", (e) => handler(e.payload)),
};
