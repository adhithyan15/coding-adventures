import { createEngramEngine } from "./engram-mosaic-host-wasm";

type HostIntent = Record<string, unknown>;
type HostResult = Record<string, unknown>;
type HostRequest = { component: string; event?: unknown };
type EngramMosaicHost = {
  platform: string;
  getProps?: (request: HostRequest) => unknown | Promise<unknown>;
  handleEvent?: (request: HostRequest) => unknown | Promise<unknown>;
};
type EngramEngine = {
  resetDemo: () => unknown;
  snapshot: () => { ok?: boolean; state?: unknown; error?: unknown };
  loadSnapshot: (snapshot: string) => { ok?: boolean; error?: unknown };
  createMosaicHost: (options: {
    deckId?: string | (() => string);
    now?: number | (() => number);
    onHostIntent?: (intent: HostIntent, result: HostResult) => unknown;
  }) => EngramMosaicHost;
};
type HostedWindow = {
  mosaicHost?: EngramMosaicHost;
};

const WASM_URL = "/engram_engine.wasm";
const DECK_ID_STORAGE_KEY = "engram.deckId";
const SNAPSHOT_STORAGE_KEY = "engram.snapshot.v1";
const HOST_INTENT_EVENT = "engram-host-intent";
const HOST_READY_EVENT = "mosaic-host-ready";

async function installEngramHost(): Promise<void> {
  const target = window as HostedWindow;
  const existing = target.mosaicHost;
  if (
    typeof existing?.getProps === "function" &&
    typeof existing?.handleEvent === "function"
  ) {
    announceHostReady();
    return;
  }

  const response = await fetch(WASM_URL);
  if (!response.ok) {
    throw new Error(`failed to fetch Engram WASM module: ${response.status}`);
  }

  const engine = createEngramEngine(await response.arrayBuffer(), {
    now: () => Date.now(),
  }) as EngramEngine;
  hydrateEngine(engine);

  const host = engine.createMosaicHost({
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
  target.mosaicHost = withSnapshotPersistence(host, engine);
  announceHostReady();
}

function hydrateEngine(engine: EngramEngine): void {
  const snapshot = readStorage(SNAPSHOT_STORAGE_KEY);
  if (snapshot) {
    const loaded = engine.loadSnapshot(snapshot);
    if (loaded.ok === true) {
      return;
    }
    console.warn("Engram persisted snapshot was invalid; resetting demo state", loaded.error);
  }

  engine.resetDemo();
  persistSnapshot(engine);
}

function withSnapshotPersistence(
  host: EngramMosaicHost,
  engine: EngramEngine,
): EngramMosaicHost {
  const handleEvent = host.handleEvent;
  if (typeof handleEvent !== "function") {
    return host;
  }

  return {
    ...host,
    async handleEvent(request: HostRequest) {
      const result = await handleEvent(request);
      if (!isErrorResult(result)) {
        persistSnapshot(engine);
      }
      return result;
    },
  };
}

function persistSnapshot(engine: EngramEngine): void {
  const snapshot = engine.snapshot();
  if (snapshot.ok !== true || snapshot.state === undefined) {
    console.warn("Engram snapshot could not be persisted", snapshot.error);
    return;
  }
  writeStorage(SNAPSHOT_STORAGE_KEY, JSON.stringify(snapshot.state));
}

function isErrorResult(result: unknown): boolean {
  return (
    typeof result === "object" &&
    result !== null &&
    "error" in result &&
    Boolean((result as { error?: unknown }).error)
  );
}

function selectedDeckId(): string {
  return readStorage(DECK_ID_STORAGE_KEY) ?? "";
}

function readStorage(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch (error) {
    console.warn(`Engram could not read ${key} from localStorage`, error);
    return null;
  }
}

function writeStorage(key: string, value: string): void {
  try {
    window.localStorage.setItem(key, value);
  } catch (error) {
    console.warn(`Engram could not write ${key} to localStorage`, error);
  }
}

function announceHostReady(): void {
  window.dispatchEvent(new CustomEvent(HOST_READY_EVENT));
}

void installEngramHost().catch((error: unknown) => {
  console.error("Engram WASM Mosaic host failed to install", error);
});
