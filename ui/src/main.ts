import { h } from "./dom";
import { startRouter, type Route } from "./router";
import { controllerPage } from "./pages/controller";
import { historyPage } from "./pages/history";
import { statsPage } from "./pages/stats";
import { onlinePage } from "./pages/online";
import { replaysPage } from "./pages/replays";
import { playButton } from "./play";

const ROUTES: Route[] = [
  { path: "/controller", title: "Controller", page: controllerPage },
  { path: "/history", title: "History", page: historyPage },
  { path: "/stats", title: "Stats", page: statsPage },
  { path: "/replays", title: "Replays", page: replaysPage },
  { path: "/online", title: "Online", page: onlinePage },
];

const nav = h("nav");
const outlet = h("main");
document.body.append(
  h("header", {}, h("div.brand", {}, h("span.mark"), "Smashcraft"), nav, playButton()),
  outlet,
);
startRouter(ROUTES, outlet, nav);
