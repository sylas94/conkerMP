// pack.js's worker: runs the extractor (dist/extract.js, crates/conker-pack-web) on the ROM it is
// sent, off the page's thread (a few seconds), and stores each file in Cache Storage under its
// assets/ URL, then `done` last. Messages back: {step, fraction}, then {done} or {error}.
const TYPES = {
  json: "application/json",
  gltf: "model/gltf+json",
  bin: "application/octet-stream",
  png: "image/png",
  wav: "audio/wav",
  md: "text/markdown; charset=utf-8",
  txt: "text/plain; charset=utf-8",
};
const type = (path) => TYPES[path.slice(path.lastIndexOf(".") + 1)] ?? "application/octet-stream";
const say = (step, fraction) => postMessage({ step, fraction });

self.onmessage = async ({ data: { extractor, cache, assets, done, rom } }) => {
  try {
    say("Loading the extractor", 0.02);
    const { default: init, pack } = await import(extractor);
    await init();
    say("Unpacking your ROM", 0.05);
    // Synchronous: no word from it until it's done.
    const files = pack(new Uint8Array(rom));
    rom = null;
    const store = await caches.open(cache);
    const n = files.len();
    // A few at a time: Cache Storage writes overlap, the files are handed out one by one.
    for (let i = 0; i < n; ) {
      const batch = [];
      for (const end = Math.min(n, i + 24); i < end; i++) {
        const path = files.path(i);
        const body = files.take(i);
        batch.push(
          store.put(
            new URL(path, assets).href,
            new Response(body, { headers: { "content-type": type(path), "content-length": String(body.length) } }),
          ),
        );
      }
      await Promise.all(batch);
      say(`Storing the files: ${i} of ${n}`, 0.6 + (0.4 * i) / n);
    }
    files.free();
    await store.put(done, new Response("1"));
    postMessage({ done: true });
  } catch (e) {
    postMessage({ error: String(e?.message ?? e) });
  }
};
