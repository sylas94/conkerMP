// The service worker (pack.js registers it): answers GETs under assets/ from the pack the player
// made from their ROM (Cache Storage), so the landing's sprites and the game's fetches find it as if
// the site served it. Anything else, and a file no pack has, goes to the network as without it.
const ASSETS = new URL("assets/", self.registration.scope).href;

self.addEventListener("install", () => self.skipWaiting());
// The first visit's page too, without a reload.
self.addEventListener("activate", (event) => event.waitUntil(self.clients.claim()));

self.addEventListener("fetch", (event) => {
  const { request } = event;
  if (request.method !== "GET" || !request.url.startsWith(ASSETS)) return;
  event.respondWith(
    caches
      .match(request.url, { ignoreSearch: true, ignoreVary: true })
      .then((stored) => stored ?? fetch(request)),
  );
});
