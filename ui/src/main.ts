import { api } from "./api";
import { h } from "./dom";
import { startRouter, type Route } from "./router";
import { controllerPage } from "./pages/controller";
import { historyPage } from "./pages/history";
import { statsPage } from "./pages/stats";
import { onlinePage } from "./pages/online";
import { replaysPage } from "./pages/replays";
import { playButton } from "./play";
import { onlineApi } from "./tauri-online";

const ROUTES: Route[] = [
  { path: "/controller", title: "Controller", page: (root) => controllerPage(root, api) },
  { path: "/history", title: "History", page: (root) => historyPage(root, api) },
  { path: "/stats", title: "Stats", page: (root) => statsPage(root, api) },
  { path: "/replays", title: "Replays", page: (root) => replaysPage(root, api) },
  { path: "/online", title: "Online", page: (root) => onlinePage(root, onlineApi) },
];

const nav = h("nav");
const outlet = h("main");
document.body.append(
  h("header", {}, h("div.brand", {}, h("span.mark"), "Smashcraft"), nav, playButton(api)),
  outlet,
);
startRouter(ROUTES, outlet, nav);
