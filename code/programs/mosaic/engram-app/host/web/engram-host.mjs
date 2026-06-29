import { installEngramMosaicHost } from "./engram-mosaic-host-wasm.mjs";

const WASM_URL = "./engram_engine.wasm";
const DECK_ID_STORAGE_KEY = "engram.deckId";
const HOST_INTENT_EVENT = "engram-host-intent";

async function installEngramHost() {
  if (
    typeof window.mosaicHost?.getProps === "function" &&
    typeof window.mosaicHost?.handleEvent === "function"
  ) {
    return;
  }

  const response = await fetch(WASM_URL);
  if (!response.ok) {
    throw new Error(`failed to fetch Engram WASM module: ${response.status}`);
  }

  installEngramMosaicHost(window, await response.arrayBuffer(), {
    deckId: selectedDeckId,
    now: () => Date.now(),
    onHostIntent: (intent, result) => {
      window.dispatchEvent(
        new CustomEvent(HOST_INTENT_EVENT, {
          detail: { intent, result },
        }),
      );
    },
  });
}

function selectedDeckId() {
  return window.localStorage.getItem(DECK_ID_STORAGE_KEY) ?? "";
}

void installEngramHost().catch(error => {
  console.error("Engram WASM Mosaic host failed to install", error);
});
