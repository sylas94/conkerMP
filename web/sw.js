// The service worker (pack.js registers it): answers GETs under assets/ from the pack the player
// made from their ROM (Cache Storage), so the landing's sprites and the game's fetches find it as if
// the site served it. Anything else, and a file no pack has, goes to the network as without it.
const ASSETS = new URL("assets/", self.registration.scope).href;

self.addEventListener("install", () => self.skipWaiting());
// The first visit's page too, without a reload.
self.addEventListener("activate", (event) => event.waitUntil(self.clients.claim()));

// The pack's cache, opened once (pack.js keeps one, named "conker-pack …"; a new build of the site
// makes another, so a miss looks again). Its files are stored under their bare URLs: one exact match
// each, where `ignoreSearch` over every cache would read all of the pack's ~2000 entries per request.
let pack = null;
async function open() {
  const name = (await caches.keys()).find((n) => n.startsWith("conker-pack"));
  return name ? caches.open(name) : null;
}
async function stored(url) {
  pack ??= open();
  let found = await (await pack)?.match(url, { ignoreVary: true });
  if (!found) {
    pack = open();
    found = await (await pack)?.match(url, { ignoreVary: true });
  }
  return found;
}

self.addEventListener("fetch", (event) => {
  const { request } = event;
  if (request.method !== "GET" || !request.url.startsWith(ASSETS)) return;
  const url = new URL(request.url);
  url.search = "";
  url.hash = "";
  event.respondWith(stored(url.href).then((found) => found ?? fetch(request)));
});
