import { Check, HardDriveUpload, Trash2 } from "lucide-react";
import { type ChangeEvent, useEffect, useRef, useState } from "react";

/** ../../pack.js, loaded from the site at run time (play.js shares it): not bundled. */
interface PackModule {
  serviceWorker(): Promise<boolean>;
  status(): Promise<"ready" | "rom" | "none">;
  build(rom: Blob, progress: (step: string, fraction: number) => void): Promise<void>;
  ensure(progress: (step: string, fraction: number) => void): Promise<boolean>;
  forget(): Promise<void>;
}
const loadPack = () =>
  import(/* @vite-ignore */ new URL("pack.js", document.baseURI).href) as Promise<PackModule>;

type State =
  | { is: "checking" }
  /** The site serves assets/ itself (`bun run serve`, deploy/'s Caddy): nothing to do here. */
  | { is: "site" }
  | { is: "none" }
  | { is: "ready" }
  | { is: "working"; step: string; fraction: number }
  | { is: "failed"; error: string };

/** "Your ROM": the game's files come from the player's own ROM, unpacked in the browser (pack.js). */
export function RomPack() {
  const [state, setState] = useState<State>({ is: "checking" });
  const [over, setOver] = useState(false);
  const progress = (step: string, fraction: number) => setState({ is: "working", step, fraction });

  useEffect(() => {
    let live = true;
    void (async () => {
      const pack = await loadPack().catch(() => null);
      if (!pack) return live && setState({ is: "site" });
      await pack.serviceWorker();
      const now = await pack.status();
      if (now === "ready") return live && setState({ is: "ready" });
      // A new build of the site: the stored ROM makes this build's pack.
      if (now === "rom") {
        await pack.ensure(progress);
        if (live) location.reload();
        return;
      }
      const served = await fetch("assets/pack.json", { method: "HEAD", cache: "no-store" }).catch(
        () => null,
      );
      if (live) setState({ is: served?.ok ? "site" : "none" });
    })().catch((e: unknown) => live && setState({ is: "failed", error: String(e) }));
    return () => {
      live = false;
    };
  }, []);

  const unpack = async (file: File | undefined) => {
    if (!file) return;
    progress("Reading your ROM", 0);
    try {
      await (await loadPack()).build(file, progress);
      // The landing's sprites come from the pack too.
      location.reload();
    } catch (e) {
      setState({ is: "failed", error: e instanceof Error ? e.message : String(e) });
    }
  };
  // A ROM dropped anywhere on the page, while one is wanted.
  const wanted = useRef(false);
  useEffect(() => {
    wanted.current = state.is === "none" || state.is === "failed";
  }, [state.is]);
  // `unpack` only sets state: the first render's is as good as any.
  useEffect(() => {
    const drag = (e: DragEvent) => {
      if (!wanted.current || !e.dataTransfer?.types.includes("Files")) return;
      e.preventDefault();
      setOver(e.type === "dragover");
    };
    const drop = (e: DragEvent) => {
      if (!wanted.current) return;
      e.preventDefault();
      setOver(false);
      void unpack(e.dataTransfer?.files[0]);
    };
    window.addEventListener("dragover", drag);
    window.addEventListener("dragleave", drag);
    window.addEventListener("drop", drop);
    return () => {
      window.removeEventListener("dragover", drag);
      window.removeEventListener("dragleave", drag);
      window.removeEventListener("drop", drop);
    };
  }, []);

  const forget = async () => {
    await (await loadPack()).forget();
    location.reload();
  };

  if (state.is === "checking" || state.is === "site") return null;
  const working = state.is === "working";
  return (
    <section aria-labelledby="rom" className="mx-auto max-w-6xl px-4 pt-12">
      <div className="board grid gap-6 p-6 md:grid-cols-[1fr_auto] md:items-center">
        <div>
          <h2 id="rom" className="cartoon flex items-center gap-2 font-display text-3xl text-amber">
            {state.is === "ready" ? (
              <Check aria-hidden className="size-7 text-squirrels" />
            ) : (
              <HardDriveUpload aria-hidden className="size-7" />
            )}
            {state.is === "ready" ? "Your ROM is in" : "Bring your own ROM"}
          </h2>
          <p className="mt-2 max-w-2xl text-bone/90">
            {state.is === "ready"
              ? "The game's maps, models and sounds are unpacked and stored in this browser. Join a game."
              : "The maps, models and sounds come from your own copy of Conker's Bad Fur Day (N64, USA). Pick the ROM file (.z64, .v64 or .n64) once: it's unpacked right here in your browser in a few seconds, and never uploaded."}
          </p>
          {working && (
            <div className="mt-4">
              <progress
                max={1}
                value={state.fraction}
                aria-label="Unpacking your ROM"
                className="block h-4 w-full overflow-hidden rounded-full border-2 border-black bg-black/50 [&::-moz-progress-bar]:bg-amber [&::-webkit-progress-bar]:bg-transparent [&::-webkit-progress-value]:bg-amber"
              />
              <output className="mt-2 block text-sm text-dust">{state.step}…</output>
            </div>
          )}
          {state.is === "failed" && (
            <p className="bubble mt-4 max-w-2xl text-sm" role="alert">
              That didn't work: {state.error}
            </p>
          )}
        </div>
        {state.is === "ready" ? (
          <button type="button" className="pill justify-self-start" onClick={forget}>
            <Trash2 aria-hidden className="size-4" />
            Forget the ROM
          </button>
        ) : (
          <label
            className={`grid cursor-pointer place-items-center gap-2 rounded-xl border-3 border-dashed p-6 text-center ${over ? "border-amber bg-amber/10" : "border-oak"} ${working ? "pointer-events-none opacity-50" : ""}`}
          >
            <span className="cta">
              <HardDriveUpload aria-hidden className="size-6" />
              Pick your ROM
            </span>
            <span className="text-sm text-dust">or drop it here</span>
            <input
              type="file"
              accept=".z64,.v64,.n64,.rom,.bin"
              className="sr-only"
              disabled={working}
              onChange={(e: ChangeEvent<HTMLInputElement>) => void unpack(e.target.files?.[0])}
            />
          </label>
        )}
      </div>
    </section>
  );
}
