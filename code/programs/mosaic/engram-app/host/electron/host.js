import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { homedir } from "node:os";
import { fileURLToPath } from "node:url";

import { createEngramEngine } from "./engram-mosaic-host-wasm.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const wasmPath = join(here, "engram_engine.wasm");
const snapshotPath =
  process.env.ENGRAM_SNAPSHOT_PATH ?? join(homedir(), ".engram", "mosaic-snapshot.v1.json");

export async function createMosaicHost() {
  const engine = createEngramEngine(readFileSync(wasmPath), {
    now: () => Date.now(),
  });
  hydrateEngine(engine);

  const host = engine.createMosaicHost({
    deckId: "",
    now: () => Date.now(),
    onHostIntent: (intent) => ({ hostIntent: intent }),
  });
  return withSnapshotPersistence(host, engine);
}

function hydrateEngine(engine) {
  if (existsSync(snapshotPath)) {
    try {
      const loaded = engine.loadSnapshot(readFileSync(snapshotPath, "utf8"));
      if (loaded.ok === true) {
        return;
      }
      console.warn("Engram persisted snapshot was invalid; resetting demo state", loaded.error);
    } catch (error) {
      console.warn("Engram could not read persisted snapshot; resetting demo state", error);
    }
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

  try {
    mkdirSync(dirname(snapshotPath), { recursive: true });
    writeFileSync(snapshotPath, JSON.stringify(snapshot.state), "utf8");
  } catch (error) {
    console.warn("Engram could not persist snapshot", error);
  }
}

function isErrorResult(result) {
  return Boolean(result && typeof result === "object" && result.error);
}
