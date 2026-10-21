// The game servers' asset pack for a GitHub Pages site: made by the same WebAssembly extractor the
// players' browsers run (web/dist/extract.js, web/build-pack.sh), so it is byte for byte theirs.
// The native `conker-extract pack` differs in the animations' last bits (the platform's sin and cos),
// and the client refuses a server whose pack fingerprint differs from its own.
//   node deploy/pack-wasm.mjs ROM OUT        (OUT must not exist: e.g. target-net/pack-wasm)
// Then upload OUT as the host's pack (deploy/README.md, "GitHub Pages"). Needs Node 20+.
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const [rom, out] = process.argv.slice(2);
if (!rom || !out) throw new Error("usage: node deploy/pack-wasm.mjs ROM OUT");
if (existsSync(out)) throw new Error(`${out} exists: pick a new folder`);
const web = join(dirname(fileURLToPath(import.meta.url)), "../web/dist");
const { initSync, pack } = await import(pathToFileURL(join(web, "extract.js")).href);
initSync({ module: readFileSync(join(web, "extract_bg.wasm")) });

const started = performance.now();
const files = pack(new Uint8Array(readFileSync(rom)));
let bytes = 0;
for (let i = 0; i < files.len(); i++) {
  const path = join(out, files.path(i));
  const data = files.take(i);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, data);
  bytes += data.length;
}
console.log(files.summary());
console.log(
  `wrote ${files.len()} files, ${bytes} bytes in ${((performance.now() - started) / 1000).toFixed(1)} s -> ${resolve(out)}`,
);
