// Hash router: #/ (home), #/channels, #/guide, #/ott, #/settings,
// #/watch/<id>, #/show/<id>, #/play/<id>.

function parse() {
  const hash = location.hash.replace(/^#/, "") || "/";
  const [, first = "", ...rest] = hash.split("/");
  return { name: first || "home", param: decodeURIComponent(rest.join("/")) };
}

export const route = $state(parse());

window.addEventListener("hashchange", () => Object.assign(route, parse()));

export function go(path) {
  location.hash = path;
}
