// The Online page's commands and events: Wisp's standalone window through its relay, Battle.net through wisp online, and Smashcraft Servers.
import type { OnlineState } from "./controller";
import { invoke, tauri } from "./api";

export const onlineApi = {
  onlineState: () => invoke<OnlineState>("online_state"),
  menuPageChoice: () => invoke<boolean | null>("menu_page_choice"),
  declineMenuPage: () => invoke<void>("decline_menu_page"),
  onlineSetup: () => invoke<OnlineState>("online_setup"),
  onlineHost: () => invoke<OnlineState>("online_host"),
  onlineJoin: (code: string) => invoke<OnlineState>("online_join", { code }),
  onlineStartNow: () => invoke<boolean>("online_start_now"),
  onlineCancel: () => invoke<void>("online_cancel"),
  smashcraftServersChoice: () => invoke<boolean | null>("smashcraft_servers_choice"),
  optInSmashcraftServers: () => invoke<void>("opt_in_smashcraft_servers"),
  relayState: () => invoke<OnlineState>("relay_state"),
  relayHost: () => invoke<OnlineState>("relay_host"),
  relayJoin: (code: string) => invoke<OnlineState>("relay_join", { code }),
  relayCancel: () => invoke<void>("relay_cancel"),
  standaloneState: () => invoke<OnlineState>("standalone_state"),
  standaloneHost: () => invoke<OnlineState>("standalone_host"),
  standaloneJoin: (code: string) => invoke<OnlineState>("standalone_join", { code }),
  standaloneCancel: () => invoke<void>("standalone_cancel"),
  onStandalone: (handler: (state: OnlineState) => void) => tauri().event.listen<OnlineState>("standalone", (e) => handler(e.payload)),
  onOnline: (handler: (state: OnlineState) => void) => tauri().event.listen<OnlineState>("online", (e) => handler(e.payload)),
  onRelay: (handler: (state: OnlineState) => void) => tauri().event.listen<OnlineState>("relay", (e) => handler(e.payload)),
};
