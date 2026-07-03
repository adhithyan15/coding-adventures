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
  exportAnkiApkg: () => { ok?: boolean; apkg?: unknown; error?: unknown };
  mergeAnkiApkg: (bytes: Uint8Array) => { ok?: boolean; state?: unknown; error?: unknown };
  createMosaicHost: (options: {
    deckId?: string | (() => string);
    now?: number | (() => number);
    onHostIntent?: (
      intent: HostIntent,
      result: HostResult,
    ) => unknown | Promise<unknown>;
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
    onHostIntent: (intent: HostIntent, result: HostResult) =>
      handleHostIntent(engine, intent, result),
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
      if (hostResultStatus(result) === "imported" && typeof host.getProps === "function") {
        const refreshed = await host.getProps({
          component: request?.component ?? "EngramApp",
        });
        const resultRecord = recordFrom(result);
        const hostResult = resultRecord.hostResult;
        return {
          ...recordFrom(refreshed),
          props: {
            ...recordFrom(recordFrom(refreshed).props),
            ...hostStatusProps(hostResult),
          },
          hostIntent: resultRecord.hostIntent,
          hostResult,
        };
      }
      return result;
    },
  };
}

async function handleHostIntent(
  engine: EngramEngine,
  intent: HostIntent,
  result: HostResult,
): Promise<HostResult> {
  const hostResult = await handleKnownHostIntent(engine, intent);
  window.dispatchEvent(
    new CustomEvent(HOST_INTENT_EVENT, {
      detail: { intent, result, hostResult },
    }),
  );
  return hostResult;
}

async function handleKnownHostIntent(
  engine: EngramEngine,
  intent: HostIntent,
): Promise<HostResult> {
  switch (intent.type) {
    case "importAnki":
      return importAnkiPackage(engine, intent);
    case "exportAnki":
      return exportAnkiPackage(engine, intent);
    default:
      return { status: "captured", hostIntent: intent };
  }
}

async function importAnkiPackage(
  engine: EngramEngine,
  intent: HostIntent,
): Promise<HostResult> {
  const file = await chooseAnkiImportFile(intent);
  if (file === null) {
    return { status: "cancelled" };
  }

  let bytes: Uint8Array;
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

async function exportAnkiPackage(
  engine: EngramEngine,
  intent: HostIntent,
): Promise<HostResult> {
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

function chooseAnkiImportFile(intent: HostIntent): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = hostIntentExtensions(intent, "accept", [".apkg", ".colpkg"]).join(",");
    input.style.position = "fixed";
    input.style.left = "-10000px";
    input.style.top = "0";

    const parent = document.body ?? document.documentElement;
    let settled = false;

    const finish = (file: File | null) => {
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

function downloadBytes(bytes: Uint8Array, name: string): void {
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

function persistSnapshot(engine: EngramEngine): void {
  const snapshot = engine.snapshot();
  if (snapshot.ok !== true || snapshot.state === undefined) {
    console.warn("Engram snapshot could not be persisted", snapshot.error);
    return;
  }
  writeStorage(SNAPSHOT_STORAGE_KEY, JSON.stringify(snapshot.state));
}

function hostFileResult(status: string, file: File, error?: unknown): HostResult {
  const result: HostResult = {
    status,
    name: file.name,
    size: file.size,
  };
  if (error !== undefined) {
    result.error = errorText(error);
  }
  return result;
}

function isErrorResult(result: unknown): boolean {
  return (
    typeof result === "object" &&
    result !== null &&
    "error" in result &&
    Boolean((result as { error?: unknown }).error)
  );
}

function hostResultStatus(result: unknown): string | undefined {
  const hostResult = recordFrom(recordFrom(result).hostResult);
  const status = hostResult.status;
  return typeof status === "string" ? status : undefined;
}

function hostStatusProps(hostResult: unknown): Record<string, unknown> {
  const record = recordFrom(hostResult);
  const status = typeof record.status === "string" ? record.status : "";
  if (!status) {
    return {};
  }
  return {
    hostStatusVisible: true,
    hostStatusKind: status,
    hostStatusLabel: hostStatusLabel(status),
    hostStatusMessage: hostStatusMessage(record, status),
  };
}

function hostStatusLabel(status: string): string {
  switch (status) {
    case "imported":
      return "Import complete";
    case "exported":
      return "Export complete";
    case "cancelled":
      return "Import cancelled";
    case "read-error":
    case "import-error":
      return "Import failed";
    case "export-error":
    case "write-error":
      return "Export failed";
    case "captured":
      return "Host action";
    default:
      return "Host status";
  }
}

function hostStatusMessage(hostResult: Record<string, unknown>, status: string): string {
  const file = hostStatusFile(hostResult);
  const error = textValue(hostResult.error);
  switch (status) {
    case "imported":
      return file ? `Imported ${file}.` : "Anki package imported.";
    case "exported":
      return file ? `Saved ${file}.` : "Anki package exported.";
    case "cancelled":
      return "No Anki package was selected.";
    case "read-error":
      return error
        ? `Could not read ${file || "the selected file"}: ${error}`
        : `Could not read ${file || "the selected file"}.`;
    case "import-error":
      return error
        ? `Could not import ${file || "the selected package"}: ${error}`
        : `Could not import ${file || "the selected package"}.`;
    case "export-error":
      return error ? `Could not export Anki package: ${error}` : "Could not export Anki package.";
    case "write-error":
      return error
        ? `Could not save ${file || "the Anki package"}: ${error}`
        : `Could not save ${file || "the Anki package"}.`;
    case "captured":
      return "Host intent captured.";
    default:
      return error || file || status;
  }
}

function hostStatusFile(hostResult: Record<string, unknown>): string {
  const name = textValue(hostResult.name);
  const size = Number(hostResult.size);
  if (name && Number.isFinite(size) && size >= 0) {
    return `${name} (${formatBytes(size)})`;
  }
  return name;
}

function formatBytes(size: number): string {
  if (size < 1024) {
    return `${size} B`;
  }
  if (size < 1024 * 1024) {
    const value = size / 1024;
    return `${value < 10 ? value.toFixed(1) : value.toFixed(0)} KB`;
  }
  const value = size / (1024 * 1024);
  return `${value < 10 ? value.toFixed(1) : value.toFixed(0)} MB`;
}

function recordFrom(value: unknown): Record<string, unknown> {
  return typeof value === "object" && value !== null ? (value as Record<string, unknown>) : {};
}

function jsonByteArray(value: unknown): Uint8Array {
  if (!Array.isArray(value)) {
    return new Uint8Array();
  }
  return Uint8Array.from(value.map((byte) => Number(byte) & 0xff));
}

function selectedDeckId(): string {
  return readStorage(DECK_ID_STORAGE_KEY) ?? "";
}

function hostIntentExtensions(
  intent: HostIntent,
  property: string,
  fallback: string[],
): string[] {
  const values = Array.isArray(intent[property]) ? intent[property] : fallback;
  const normalized = values
    .map((value) => String(value).trim())
    .filter(Boolean)
    .map((value) => (value.startsWith(".") ? value : `.${value}`));
  return normalized.length === 0 ? fallback : normalized;
}

function suggestedAnkiFileName(intent: HostIntent): string {
  const raw = String(intent.deckId ?? "engram-collection").trim() || "engram-collection";
  const safe = raw.replace(/[\/\\:*?"<>|]/g, "-");
  return safe.toLowerCase().endsWith(".apkg") ? safe : `${safe}.apkg`;
}

function errorText(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error ?? "unknown error");
}

function textValue(value: unknown): string {
  if (value === undefined || value === null) {
    return "";
  }
  return typeof value === "string" ? value : String(value);
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
