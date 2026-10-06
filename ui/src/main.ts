import { h } from "./dom";
import { startRouter, type Route } from "./router";
import { controllerPage } from "./pages/controller";
import { historyPage } from "./pages/history";
import { statsPage } from "./pages/stats";
import { playButton } from "./play";

const ROUTES: Route[] = [
  { path: "/controller", title: "Controller", page: controllerPage },
  { path: "/history", title: "History", page: historyPage },
  { path: "/stats", title: "Stats", page: statsPage },
];

const nav = h("nav");
const outlet = h("main");
document.body.append(
  h("header", {}, h("div.brand", {}, h("span.mark"), "Smashcraft"), nav, playButton()),
  outlet,
);
startRouter(ROUTES, outlet, nav);
