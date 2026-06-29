// Dependency-free JavaScript loader for the Engram WASM ABI.
//
// The Rust module exports the repo-standard linear-memory protocol. This file
// presents two layers:
//   1. createEngramEngine(wasmBytes): direct JSON facade calls.
//   2. createEngramMosaicHost(wasmBytes): window.mosaicHost-shaped adapter for
//      generated Mosaic React/Electron shells.

export function createEngramEngine(wasmBytes, options = {}) {
  const module =
    wasmBytes instanceof WebAssembly.Module ? wasmBytes : new WebAssembly.Module(wasmBytes);
  const instance = new WebAssembly.Instance(module, {});
  const ex = instance.exports;
  const enc = new TextEncoder();
  const dec = new TextDecoder();

  const mem = () => new Uint8Array(ex.memory.buffer);

  function writeStr(value) {
    const bytes = enc.encode(String(value));
    if (bytes.length === 0) return [0, 0];
    const ptr = ex.alloc(bytes.length);
    if (ptr === 0) {
      throw new Error("engram-wasm alloc returned null");
    }
    mem().set(bytes, ptr);
    return [ptr, bytes.length];
  }

  function freeInput(ptr, len) {
    if (len) ex.dealloc(ptr, len);
  }

  function readResult(ptr) {
    if (ptr === 0) {
      throw new Error("engram-wasm returned null");
    }
    const bytes = mem();
    const len =
      (bytes[ptr] | (bytes[ptr + 1] << 8) | (bytes[ptr + 2] << 16) | (bytes[ptr + 3] << 24)) >>>
      0;
    const value = dec.decode(bytes.subarray(ptr + 4, ptr + 4 + len));
    ex.dealloc(ptr, 4 + len);
    return value;
  }

  function jsonResult(raw) {
    return JSON.parse(raw);
  }

  function call0(fn) {
    return readResult(ex[fn]());
  }

  function call1(fn, value) {
    const [ptr, len] = writeStr(value);
    const result = ex[fn](ptr, len);
    freeInput(ptr, len);
    return readResult(result);
  }

  function callStrU64(fn, value, number) {
    const [ptr, len] = writeStr(value);
    const result = ex[fn](ptr, len, toBigIntMs(number));
    freeInput(ptr, len);
    return readResult(result);
  }

  function callStrU64U64(fn, value, first, second) {
    const [ptr, len] = writeStr(value);
    const result = ex[fn](ptr, len, toBigIntMs(first), toBigIntMs(second));
    freeInput(ptr, len);
    return readResult(result);
  }

  function callEventDeckNow(fn, event, deckId, now) {
    const [ep, el] = writeStr(serializeEvent(event));
    const [dp, dl] = writeStr(deckId);
    const result = ex[fn](ep, el, dp, dl, toBigIntMs(now));
    freeInput(ep, el);
    freeInput(dp, dl);
    return readResult(result);
  }

  function currentDeckId() {
    const value = typeof options.deckId === "function" ? options.deckId() : options.deckId;
    return value === undefined || value === null ? "" : String(value);
  }

  function currentNow() {
    return typeof options.now === "function" ? options.now() : Date.now();
  }

  return {
    reset: () => ex.reset(),
    snapshot: () => jsonResult(call0("snapshot")),
    getState: () => jsonResult(call0("get_state")),
    loadSnapshot: (snapshot) => jsonResult(call1("load_snapshot", stringifyInput(snapshot))),
    dispatch: (command) => jsonResult(call1("dispatch", stringifyInput(command))),
    buildQueue: (deckId = currentDeckId(), now = currentNow()) =>
      jsonResult(callStrU64("build_queue", deckId, now)),
    getDeckStats: (deckId = currentDeckId(), now = currentNow()) =>
      jsonResult(callStrU64("get_deck_stats", deckId, now)),
    sessionProgress: () => jsonResult(call0("session_progress")),
    reviewHistory: (deckId = currentDeckId(), reviewedAfter = 0, reviewedBefore = currentNow()) =>
      jsonResult(callStrU64U64("review_history", deckId, reviewedAfter, reviewedBefore)),
    searchCards: (query, now = currentNow()) => jsonResult(callStrU64("search_cards", query, now)),
    engramAppProps: (deckId = currentDeckId(), now = currentNow()) =>
      jsonResult(callStrU64("engram_app_props", deckId, now)),
    engramBrowserProps: (query, now = currentNow()) =>
      jsonResult(callStrU64("engram_browser_props", query, now)),
    handleEngramAppEvent: (event, deckId = currentDeckId(), now = currentNow()) =>
      jsonResult(callEventDeckNow("handle_engram_app_event", event, deckId, now)),
    createMosaicHost: (hostOptions = {}) =>
      createMosaicHost({
        getProps: (request) => {
          assertEngramComponent(request);
          const deck = valueOf(hostOptions.deckId ?? currentDeckId);
          const now = valueOf(hostOptions.now ?? currentNow);
          return toHostResponse(jsonResult(callStrU64("engram_app_props", deck, valueOf(now))));
        },
        handleEvent: async (request) => {
          assertEngramComponent(request);
          const deck = valueOf(hostOptions.deckId ?? currentDeckId);
          const now = valueOf(hostOptions.now ?? currentNow);
          const result = jsonResult(
            callEventDeckNow("handle_engram_app_event", request.event, deck, valueOf(now)),
          );
          const response = toHostResponse(result);
          if (result.hostIntent && typeof hostOptions.onHostIntent === "function") {
            const hostResult = await hostOptions.onHostIntent(result.hostIntent, result);
            if (hostResult !== undefined) {
              response.hostResult = hostResult;
            }
          }
          return response;
        },
      }),
  };
}

export function createEngramMosaicHost(wasmBytes, options = {}) {
  return createEngramEngine(wasmBytes, options).createMosaicHost(options);
}

export async function createEngramMosaicHostFromUrl(url, options = {}) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`failed to fetch Engram WASM module: ${response.status}`);
  }
  return createEngramMosaicHost(await response.arrayBuffer(), options);
}

export function installEngramMosaicHost(targetWindow, wasmBytes, options = {}) {
  const host = createEngramMosaicHost(wasmBytes, options);
  targetWindow.mosaicHost = host;
  dispatchMosaicHostReady(targetWindow, host);
  return host;
}

export function camelCaseMosaicSlots(props) {
  const out = {};
  for (const [key, value] of Object.entries(props ?? {})) {
    out[toCamelCaseFirstLower(key)] = value;
  }
  return out;
}

export function toCamelCaseFirstLower(name) {
  const raw = String(name);
  const camel = raw.replace(/-([a-z0-9])/g, (_match, next) => next.toUpperCase());
  return camel.length === 0 ? camel : camel[0].toLowerCase() + camel.slice(1);
}

function createMosaicHost(host) {
  return {
    platform: "engram-wasm",
    getProps: host.getProps,
    handleEvent: host.handleEvent,
  };
}

function dispatchMosaicHostReady(targetWindow, host) {
  if (
    typeof targetWindow?.dispatchEvent !== "function" ||
    typeof targetWindow?.CustomEvent !== "function"
  ) {
    return;
  }
  targetWindow.dispatchEvent(
    new targetWindow.CustomEvent("mosaic-host-ready", {
      detail: { platform: host.platform },
    }),
  );
}

function toHostResponse(result) {
  if (!result || result.ok !== true) {
    return { props: {}, error: result?.error ?? "Engram host returned an error" };
  }
  const response = { props: camelCaseMosaicSlots(result.props ?? {}) };
  if (result.hostIntent) {
    response.hostIntent = result.hostIntent;
  }
  if (result.event) {
    response.event = result.event;
  }
  return response;
}

function stringifyInput(value) {
  return typeof value === "string" ? value : JSON.stringify(value);
}

function serializeEvent(event) {
  return typeof event === "string" ? event : JSON.stringify(event);
}

function toBigIntMs(value) {
  return BigInt(Math.trunc(Number(value)));
}

function valueOf(valueOrFn) {
  return typeof valueOrFn === "function" ? valueOrFn() : valueOrFn;
}

function assertEngramComponent(request) {
  if (request?.component && request.component !== "EngramApp") {
    throw new Error(`Engram WASM host cannot serve component ${request.component}`);
  }
}
