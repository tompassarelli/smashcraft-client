// Hash routes. A page is a function that fills its container and returns its cleanup.
// New pages (history, stats, replays, online) are one entry in ROUTES.

export type Page = (root: HTMLElement) => () => void;
export type Route = { path: string; title: string; page: Page };

export function resolve(routes: Route[], hash: string): Route {
  const path = hash.replace(/^#/, "") || "/";
  return routes.find((route) => route.path === path) ?? routes[0]!;
}

export function startRouter(routes: Route[], outlet: HTMLElement, nav: HTMLElement): void {
  let cleanup: (() => void) | undefined;
  const links = routes.map((route) => {
    const a = document.createElement("a");
    a.href = `#${route.path}`;
    a.textContent = route.title;
    nav.append(a);
    return a;
  });
  const show = () => {
    const route = resolve(routes, location.hash);
    cleanup?.();
    outlet.replaceChildren();
    links.forEach((a, i) => a.classList.toggle("active", routes[i] === route));
    document.title = `Smashcraft · ${route.title}`;
    cleanup = route.page(outlet);
  };
  window.addEventListener("hashchange", show);
  show();
}
