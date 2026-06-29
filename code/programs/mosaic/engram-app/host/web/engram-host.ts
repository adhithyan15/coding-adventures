import { installEngramMosaicHost } from "./engram-mosaic-host-wasm";

type HostIntent = Record<string, unknown>;
type HostResult = Record<string, unknown>;
type HostedWindow = Window & {
  mosaicHost?: {
    getProps?: unknown;
    handleEvent?: unknown;
  };
};

const WASM_URL = "/engram_engine.wasm";
const DECK_ID_STORAGE_KEY = "engram.deckId";
const HOST_INTENT_EVENT = "engram-host-intent";

async function installEngramHost(): Promise<void> {
  const target = window as HostedWindow;
  const existing = target.mosaicHost;
  if (
    typeof existing?.getProps === "function" &&
    typeof existing?.handleEvent === "function"
  ) {
    return;
  }

  const response = await fetch(WASM_URL);
  if (!response.ok) {
    throw new Error(`failed to fetch Engram WASM module: ${response.status}`);
  }

  installEngramMosaicHost(target, await response.arrayBuffer(), {
    deckId: selectedDeckId,
    now: () => Date.now(),
    onHostIntent: (intent: HostIntent, result: HostResult) => {
      window.dispatchEvent(
        new CustomEvent(HOST_INTENT_EVENT, {
          detail: { intent, result },
        }),
      );
    },
  });
}

function selectedDeckId(): string {
  return window.localStorage.getItem(DECK_ID_STORAGE_KEY) ?? "";
}

void installEngramHost().catch((error: unknown) => {
  console.error("Engram WASM Mosaic host failed to install", error);
});
