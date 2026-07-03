import { dialog } from "electron";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, extname, join } from "node:path";
import { homedir } from "node:os";
import { fileURLToPath } from "node:url";

import { createEngramEngine } from "./engram-mosaic-host-wasm.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const wasmPath = join(here, "engram_engine.wasm");
const snapshotPath =
  process.env.ENGRAM_SNAPSHOT_PATH ?? join(homedir(), ".engram", "mosaic-snapshot.v1.json");
const sidecarPath =
  process.env.ENGRAM_HOST_CLI ??
  join(here, process.platform === "win32" ? "engram-host-cli.exe" : "engram-host-cli");

export async function createMosaicHost() {
  const engine = createEngramEngine(readFileSync(wasmPath), {
    now: () => Date.now(),
  });
  hydrateEngine(engine);

  const host = engine.createMosaicHost({
    deckId: "",
    now: () => Date.now(),
    onHostIntent: (intent) => handleHostIntent(engine, intent),
  });
  return withSnapshotPersistence(host, engine);
}

function hydrateEngine(engine) {
  if (loadPersistedSnapshot(engine)) {
    return;
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
      if (result?.hostResult?.status === "imported" && typeof host.getProps === "function") {
        const refreshed = await host.getProps({ component: request?.component ?? "EngramApp" });
        return {
          ...refreshed,
          hostIntent: result.hostIntent,
          hostResult: result.hostResult,
        };
      }
      return result;
    },
  };
}

async function handleHostIntent(engine, intent) {
  if (intent?.type === "importAnki") {
    return importAnkiPackage(engine, intent);
  }
  if (intent?.type === "exportAnki") {
    return exportAnkiPackage(engine, intent);
  }
  return { status: "captured", hostIntent: intent };
}

async function importAnkiPackage(engine, intent) {
  const { canceled, filePaths } = await dialog.showOpenDialog({
    title: "Import Anki package",
    properties: ["openFile"],
    filters: [
      {
        name: "Anki packages",
        extensions: fileTypes(hostIntentExtensions(intent, "accept", [".apkg", ".colpkg"])),
      },
    ],
  });
  const filePath = filePaths?.[0];
  if (canceled || !filePath) {
    return { status: "cancelled" };
  }

  try {
    persistSnapshot(engine);
    const merged = runSidecar(["merge-apkg", snapshotPath, filePath]);
    if (merged.ok !== true) return sidecarError("import-error", filePath, merged);
    if (!loadPersistedSnapshot(engine)) return sidecarError("snapshot-error", filePath, merged);
    return { status: "imported", path: filePath };
  } catch (error) {
    console.warn("Engram could not import Anki package", error);
    return { status: "read-error", path: filePath, error: String(error?.message ?? error) };
  }
}

async function exportAnkiPackage(engine, intent) {
  const { canceled, filePath } = await dialog.showSaveDialog({
    title: "Export Anki package",
    defaultPath: join(homedir(), suggestedAnkiFileName(intent)),
    filters: [
      {
        name: "Anki packages",
        extensions: fileTypes(hostIntentExtensions(intent, "extensions", [".apkg"])),
      },
    ],
  });
  if (canceled || !filePath) {
    return { status: "cancelled" };
  }

  const outputPath = extname(filePath) ? filePath : `${filePath}.apkg`;
  try {
    persistSnapshot(engine);
    const exported = runSidecar(["export-apkg", snapshotPath, outputPath]);
    if (exported.ok !== true) return sidecarError("export-error", outputPath, exported);
    return { status: "exported", path: outputPath };
  } catch (error) {
    console.warn("Engram could not export Anki package", error);
    return { status: "write-error", path: outputPath, error: String(error?.message ?? error) };
  }
}

function loadPersistedSnapshot(engine) {
  if (!existsSync(snapshotPath)) {
    return false;
  }
  try {
    const loaded = engine.loadSnapshot(readFileSync(snapshotPath, "utf8"));
    if (loaded.ok === true) {
      return true;
    }
    console.warn("Engram persisted snapshot was invalid", loaded.error);
  } catch (error) {
    console.warn("Engram could not read persisted snapshot", error);
  }
  return false;
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

function runSidecar(args) {
  if (!existsSync(sidecarPath)) {
    return {
      ok: false,
      error: `Engram native sidecar not found at ${sidecarPath}`,
    };
  }
  const result = spawnSync(sidecarPath, args, {
    encoding: "utf8",
    windowsHide: true,
  });
  if (result.error) {
    return { ok: false, error: String(result.error.message ?? result.error) };
  }
  const raw = String(result.stdout ?? "").trim();
  let parsed = null;
  if (raw) {
    try {
      parsed = JSON.parse(raw.split(/\r?\n/).at(-1));
    } catch (error) {
      return { ok: false, error: `Engram native sidecar returned invalid JSON: ${error}` };
    }
  }
  if (result.status !== 0) {
    return parsed ?? { ok: false, error: String(result.stderr ?? "Engram native sidecar failed") };
  }
  return parsed ?? { ok: false, error: "Engram native sidecar returned no JSON" };
}

function sidecarError(status, path, result) {
  return {
    status,
    path,
    error: result?.error ?? "Engram native sidecar failed",
  };
}

function hostIntentExtensions(intent, property, fallback) {
  const values = Array.isArray(intent?.[property]) ? intent[property] : fallback;
  const normalized = values
    .map((value) => String(value).trim())
    .filter(Boolean)
    .map((value) => (value.startsWith(".") ? value : `.${value}`));
  return normalized.length === 0 ? fallback : normalized;
}

function fileTypes(extensions) {
  return extensions.map((extension) =>
    extension.startsWith(".") ? extension.slice(1) : extension,
  );
}

function suggestedAnkiFileName(intent) {
  const raw = String(intent?.deckId ?? "engram-collection").trim() || "engram-collection";
  const safe = raw.replace(/[\/\\:*?"<>|]/g, "-");
  return safe.toLowerCase().endsWith(".apkg") ? safe : `${safe}.apkg`;
}
