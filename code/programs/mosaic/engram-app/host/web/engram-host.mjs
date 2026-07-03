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
    onHostIntent: (intent, result) => handleHostIntent(engine, intent, result),
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
      if (hostResultStatus(result) === "imported" && typeof host.getProps === "function") {
        const refreshed = await host.getProps({
          component: request?.component ?? "EngramApp",
        });
        return {
          ...recordFrom(refreshed),
          hostIntent: recordFrom(result).hostIntent,
          hostResult: recordFrom(result).hostResult,
        };
      }
      return result;
    },
  };
}

async function handleHostIntent(engine, intent, result) {
  const hostResult = await handleKnownHostIntent(engine, intent);
  window.dispatchEvent(
    new CustomEvent(HOST_INTENT_EVENT, {
      detail: { intent, result, hostResult },
    }),
  );
  return hostResult;
}

async function handleKnownHostIntent(engine, intent) {
  if (intent?.type === "importAnki") {
    return importAnkiPackage(engine, intent);
  }
  if (intent?.type === "exportAnki") {
    return exportAnkiPackage(engine, intent);
  }
  return { status: "captured", hostIntent: intent };
}

async function importAnkiPackage(engine, intent) {
  const file = await chooseAnkiImportFile(intent);
  if (file === null) {
    return { status: "cancelled" };
  }

  let bytes;
  try {
    bytes = new Uint8Array(await file.arrayBuffer());
  } catch (error) {
    return hostFileResult("read-error", file, error);
  }

  if (bytes.length === 0) {
    return hostFileResult("import-error", file, "Anki package was empty");
  }

  const imported = engine.mergeAnkiApkg(bytes);
  if (imported.ok !== true) {
    return hostFileResult("import-error", file, imported.error);
  }

  return {
    status: "imported",
    name: file.name,
    size: file.size,
  };
}

async function exportAnkiPackage(engine, intent) {
  const exported = engine.exportAnkiApkg();
  if (exported.ok !== true) {
    return {
      status: "export-error",
      error: errorText(exported.error),
    };
  }

  const bytes = jsonByteArray(exported.apkg);
  if (bytes.length === 0) {
    return {
      status: "export-error",
      error: "Engram WASM host returned an empty APKG",
    };
  }

  const name = suggestedAnkiFileName(intent);
  try {
    downloadBytes(bytes, name);
  } catch (error) {
    return {
      status: "write-error",
      name,
      error: errorText(error),
    };
  }

  return {
    status: "exported",
    name,
    size: bytes.length,
  };
}

function chooseAnkiImportFile(intent) {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = hostIntentExtensions(intent, "accept", [".apkg", ".colpkg"]).join(",");
    input.style.position = "fixed";
    input.style.left = "-10000px";
    input.style.top = "0";

    const parent = document.body ?? document.documentElement;
    let settled = false;

    const finish = (file) => {
      if (settled) return;
      settled = true;
      window.removeEventListener("focus", onFocus);
      input.remove();
      resolve(file);
    };

    const onFocus = () => {
      window.setTimeout(() => {
        if (!settled && (input.files?.length ?? 0) === 0) {
          finish(null);
        }
      }, 300);
    };

    input.addEventListener(
      "change",
      () => {
        finish(input.files?.item(0) ?? null);
      },
      { once: true },
    );
    input.addEventListener("cancel", () => finish(null), { once: true });
    window.addEventListener("focus", onFocus, { once: true });
    parent.appendChild(input);
    input.click();
  });
}

function downloadBytes(bytes, name) {
  const blob = new Blob([bytes], { type: "application/octet-stream" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.style.display = "none";

  const parent = document.body ?? document.documentElement;
  parent.appendChild(link);
  link.click();
  link.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 1000);
}

function persistSnapshot(engine) {
  const snapshot = engine.snapshot();
  if (snapshot.ok !== true || snapshot.state === undefined) {
    console.warn("Engram snapshot could not be persisted", snapshot.error);
    return;
  }
  writeStorage(SNAPSHOT_STORAGE_KEY, JSON.stringify(snapshot.state));
}

function hostFileResult(status, file, error) {
  const result = {
    status,
    name: file.name,
    size: file.size,
  };
  if (error !== undefined) {
    result.error = errorText(error);
  }
  return result;
}

function isErrorResult(result) {
  return Boolean(result && typeof result === "object" && result.error);
}

function hostResultStatus(result) {
  const status = recordFrom(recordFrom(result).hostResult).status;
  return typeof status === "string" ? status : undefined;
}

function recordFrom(value) {
  return typeof value === "object" && value !== null ? value : {};
}

function jsonByteArray(value) {
  if (!Array.isArray(value)) {
    return new Uint8Array();
  }
  return Uint8Array.from(value.map((byte) => Number(byte) & 0xff));
}

function selectedDeckId() {
  return readStorage(DECK_ID_STORAGE_KEY) ?? "";
}

function hostIntentExtensions(intent, property, fallback) {
  const values = Array.isArray(intent?.[property]) ? intent[property] : fallback;
  const normalized = values
    .map((value) => String(value).trim())
    .filter(Boolean)
    .map((value) => (value.startsWith(".") ? value : `.${value}`));
  return normalized.length === 0 ? fallback : normalized;
}

function suggestedAnkiFileName(intent) {
  const raw = String(intent?.deckId ?? "engram-collection").trim() || "engram-collection";
  const safe = raw.replace(/[\/\\:*?"<>|]/g, "-");
  return safe.toLowerCase().endsWith(".apkg") ? safe : `${safe}.apkg`;
}

function errorText(error) {
  return error instanceof Error ? error.message : String(error ?? "unknown error");
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
