// The asset pack in the browser, for a site that doesn't serve it (GitHub Pages: README.md, "Asset
// pack in the browser"). The player picks their own Conker's Bad Fur Day (USA) ROM once:
// pack-worker.js runs the extractor (crates/conker-pack-web, dist/extract.js) on it and stores every
// file in Cache Storage, and sw.js, the service worker, answers the page's and the game's requests
// for assets/… from there. The ROM is kept too, in its own cache: when a new build of the site brings
// a new extractor, the pack is made again from it without asking.
// On a site that serves assets/ itself (`bun run serve`, deploy/'s Caddy) nothing is stored: the
// requests the service worker doesn't answer go to the network as before.

// deploy/pages.sh moves the extractor to ./x/HASH/: its path names the pack it makes.
const EXTRACTOR = new URL("./dist/extract.js", import.meta.url);
const PACK = `conker-pack ${EXTRACTOR.pathname}`;
const ROM = "conker-rom";
const ASSETS = new URL("assets/", import.meta.url);
const ROM_KEY = new URL("rom.z64", import.meta.url).href;
// The last file written: a pack without it was cut short.
const DONE = new URL(".complete", ASSETS).href;

/** Registers sw.js and waits until it controls this page; false without service workers. */
export async function serviceWorker() {
  if (!("serviceWorker" in navigator)) return false;
  try {
    await navigator.serviceWorker.register(new URL("sw.js", import.meta.url));
    await navigator.serviceWorker.ready;
    // A first visit: the worker claims the page as it activates (sw.js).
    if (!navigator.serviceWorker.controller)
      await new Promise((ok) => {
        navigator.serviceWorker.addEventListener("controllerchange", ok, { once: true });
        setTimeout(ok, 3000);
      });
    return Boolean(navigator.serviceWorker.controller);
  } catch {
    return false;
  }
}

const hasCaches = () => typeof caches !== "undefined";

/** "ready" (this build's pack is stored), "rom" (the ROM is, the pack needs making again) or "none". */
export async function status() {
  if (!hasCaches()) return "none";
  if (await caches.has(PACK)) {
    if (await (await caches.open(PACK)).match(DONE)) return "ready";
  }
  return (await (await caches.open(ROM)).match(ROM_KEY)) ? "rom" : "none";
}

/** Old builds' packs and a cut-short one of this build: never served again. */
async function dropPacks(keep) {
  for (const name of await caches.keys())
    if (name.startsWith("conker-pack") && name !== keep) await caches.delete(name);
}

/**
 * Makes the pack from `rom` (a File, Blob or ArrayBuffer) and stores it, the ROM with it.
 * `progress(step, fraction)` follows it. Throws the extractor's error (a wrong ROM: its SHA-1).
 */
export async function build(rom, progress = () => {}) {
  if (!hasCaches()) throw new Error("this browser can't store files for the site (a private window?)");
  const bytes = rom instanceof ArrayBuffer ? rom : await rom.arrayBuffer();
  await dropPacks(null);
  const worker = new Worker(new URL("pack-worker.js", import.meta.url), { type: "module" });
  try {
    await new Promise((ok, fail) => {
      worker.onmessage = ({ data }) => {
        if (data.error) fail(new Error(data.error));
        else if (data.done) ok();
        else progress(data.step, data.fraction);
      };
      worker.onerror = (e) => fail(new Error(e.message || "the extractor failed to start"));
      // A copy goes over (moved, not copied again): `bytes` is stored below.
      const copy = bytes.slice(0);
      worker.postMessage(
        { extractor: EXTRACTOR.href, cache: PACK, assets: ASSETS.href, done: DONE, rom: copy },
        [copy],
      );
    });
  } catch (e) {
    await dropPacks(null);
    throw e;
  } finally {
    worker.terminate();
  }
  await (await caches.open(ROM)).put(
    ROM_KEY,
    new Response(bytes, { headers: { "content-type": "application/octet-stream" } }),
  );
}

/** Forgets the ROM and every pack made from it. */
export async function forget() {
  if (!hasCaches()) return;
  await dropPacks(null);
  await caches.delete(ROM);
}

/**
 * Before the game starts: true once assets/ answers, from this build's pack (made again from the
 * stored ROM if a new build came) or from the site itself; false if the player has to pick a ROM.
 */
export async function ensure(progress = () => {}) {
  await serviceWorker();
  const now = await status();
  if (now === "ready") return true;
  if (now === "rom") {
    const rom = await (await caches.open(ROM)).match(ROM_KEY);
    await build(await rom.arrayBuffer(), progress);
    return true;
  }
  if (hasCaches()) await dropPacks(null);
  const res = await fetch(new URL("pack.json", ASSETS), { method: "HEAD", cache: "no-store" }).catch(
    () => null,
  );
  return Boolean(res?.ok);
}
