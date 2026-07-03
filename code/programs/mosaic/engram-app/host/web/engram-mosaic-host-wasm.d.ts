export type EngramMosaicHost = {
  platform: string;
  getProps?: (request: { component: string }) => unknown | Promise<unknown>;
  handleEvent?: (request: { component: string; event?: unknown }) => unknown | Promise<unknown>;
};

export type EngramEngine = {
  reset: () => unknown;
  resetDemo: () => unknown;
  demoSnapshot: () => unknown;
  snapshot: () => { ok?: boolean; state?: unknown; error?: unknown };
  getState: () => unknown;
  loadSnapshot: (snapshot: unknown) => { ok?: boolean; error?: unknown };
  dispatch: (command: unknown) => unknown;
  exportAnkiApkg: () => { ok?: boolean; apkg?: unknown; error?: unknown };
  mergeAnkiApkg: (bytes: Uint8Array) => { ok?: boolean; state?: unknown; error?: unknown };
  createMosaicHost: (options?: {
    deckId?: string | (() => string);
    now?: number | (() => number);
    onHostIntent?: (
      intent: Record<string, unknown>,
      result: Record<string, unknown>,
    ) => unknown | Promise<unknown>;
  }) => EngramMosaicHost;
};

export function createEngramEngine(
  wasmBytes: BufferSource | WebAssembly.Module,
  options?: {
    demo?: boolean;
    deckId?: string | (() => string);
    now?: number | (() => number);
  },
): EngramEngine;

export function installEngramMosaicHost(
  targetWindow: Window,
  wasmBytes: BufferSource | WebAssembly.Module,
  options?: {
    demo?: boolean;
    deckId?: string | (() => string);
    now?: number | (() => number);
    onHostIntent?: (
      intent: Record<string, unknown>,
      result: Record<string, unknown>,
    ) => unknown | Promise<unknown>;
  },
): EngramMosaicHost;
