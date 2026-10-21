//! `conker-extract pack` compiled to WebAssembly: web/pack-worker.js hands it the player's ROM and
//! stores the files it returns in the browser's Cache Storage, where web/sw.js serves them as
//! `assets/…`. The same maps as `bun run pack` (package.json), without the background recordings
//! (they come from `lab/audio/`, not the ROM), so the pack is the one the game servers read: the
//! client checks its fingerprint against the server's `/info`.

use wasm_bindgen::prelude::*;

/// `bun run pack`'s maps, in its order.
pub const MAPS: [usize; 8] = [0x2D, 0x30, 0x34, 0x3F, 0x2B, 0x1A, 0x24, 0x33];

/// The exported pack: its files, read one at a time so the page never holds two copies of it.
#[wasm_bindgen]
pub struct Pack {
    files: Vec<(String, Vec<u8>)>,
    summary: Vec<String>,
}

#[wasm_bindgen]
impl Pack {
    /// How many files.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// File `i`'s path under `assets/` (`maps/2d/level.gltf`).
    pub fn path(&self, i: usize) -> String {
        self.files[i].0.clone()
    }

    /// File `i`'s bytes. Each is handed out once: its memory is freed after.
    pub fn take(&mut self, i: usize) -> Vec<u8> {
        std::mem::take(&mut self.files[i].1)
    }

    /// The extractor's console lines.
    pub fn summary(&self) -> String {
        self.summary.join("\n")
    }
}

/// Exports the asset pack from a Conker's Bad Fur Day (USA) ROM in any byte order (.z64, .v64,
/// .n64); any other ROM is refused with its SHA-1.
#[wasm_bindgen]
pub fn pack(rom: Vec<u8>) -> Result<Pack, JsError> {
    let rom = extract_core::Rom::new(rom).map_err(|e| JsError::new(&e.to_string()))?;
    let maps: Vec<_> = (MAPS.iter())
        .map(|&scene| conker_extract::PackMap {
            scene,
            background: None,
            song: None,
            stem: None,
        })
        .collect();
    let export = conker_extract::pack(&rom, &maps).map_err(|e| JsError::new(&format!("{e:#}")))?;
    Ok(Pack {
        files: export.files,
        summary: export.summary,
    })
}
