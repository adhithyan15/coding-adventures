import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { createEngramMosaicHost } from "./engram-mosaic-host-wasm.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const wasmPath = join(here, "engram_engine.wasm");

export async function createMosaicHost() {
  return createEngramMosaicHost(readFileSync(wasmPath), {
    deckId: "",
    now: () => Date.now(),
    onHostIntent: (intent) => ({ hostIntent: intent }),
  });
}
