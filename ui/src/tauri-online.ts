// The Online page's commands and events: Wisp's standalone window through its relay.
import type { OnlineState } from "./controller";
import { invoke, tauri } from "./api";

export const onlineApi = {
  standaloneState: () => invoke<OnlineState>("standalone_state"),
  standaloneHost: () => invoke<OnlineState>("standalone_host"),
  standaloneJoin: (code: string) => invoke<OnlineState>("standalone_join", { code }),
  standaloneCancel: () => invoke<void>("standalone_cancel"),
  onStandalone: (handler: (state: OnlineState) => void) => tauri().event.listen<OnlineState>("standalone", (e) => handler(e.payload)),
};

export type OnlineApi = typeof onlineApi;
