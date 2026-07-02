import { createEngramEngine } from "./engram-mosaic-host-wasm.mjs";

const WASM_URL = "./engram_engine.wasm";
const DECK_ID_STORAGE_KEY = "engram.deckId";
const SNAPSHOT_STORAGE_KEY = "engram.snapshot.v1";
const HOST_INTENT_EVENT = "engram-host-intent";
const HOST_READY_EVENT = "mosaic-host-ready";

async function installEngramHost() {
  if (
    typeof window.mosaicHost?.getProps === "function" &&
    typeof window.mosaicHost?.handleEvent === "function"
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
  });
  hydrateEngine(engine);

  const host = engine.createMosaicHost({
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
  window.mosaicHost = withSnapshotPersistence(host, engine);
  announceHostReady();
}

function hydrateEngine(engine) {
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

function withSnapshotPersistence(host, engine) {
  const handleEvent = host.handleEvent;
  if (typeof handleEvent !== "function") {
    return host;
  }

  return {
    ...host,
    async handleEvent(request) {
      const result = await handleEvent(request);
      if (!isErrorResult(result)) {
        persistSnapshot(engine);
      }
      return result;
    },
  };
}

function persistSnapshot(engine) {
  const snapshot = engine.snapshot();
  if (snapshot.ok !== true || snapshot.state === undefined) {
    console.warn("Engram snapshot could not be persisted", snapshot.error);
    return;
  }
  writeStorage(SNAPSHOT_STORAGE_KEY, JSON.stringify(snapshot.state));
}

function isErrorResult(result) {
  return Boolean(result && typeof result === "object" && result.error);
}

function selectedDeckId() {
  return readStorage(DECK_ID_STORAGE_KEY) ?? "";
}

function readStorage(key) {
  try {
    return window.localStorage.getItem(key);
  } catch (error) {
    console.warn(`Engram could not read ${key} from localStorage`, error);
    return null;
  }
}

function writeStorage(key, value) {
  try {
    window.localStorage.setItem(key, value);
  } catch (error) {
    console.warn(`Engram could not write ${key} to localStorage`, error);
  }
}

function announceHostReady() {
  window.dispatchEvent(new CustomEvent(HOST_READY_EVENT));
}

void installEngramHost().catch(error => {
  console.error("Engram WASM Mosaic host failed to install", error);
});
