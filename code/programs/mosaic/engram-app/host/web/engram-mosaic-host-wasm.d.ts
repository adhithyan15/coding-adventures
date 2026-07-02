export type EngramMosaicHost = {
  platform: string;
  getProps?: (request: { component: string }) => unknown | Promise<unknown>;
  handleEvent?: (request: { component: string; event?: unknown }) => unknown | Promise<unknown>;
};

export function installEngramMosaicHost(
  targetWindow: Window,
  wasmBytes: BufferSource | WebAssembly.Module,
  options?: {
    demo?: boolean;
    deckId?: string | (() => string);
    now?: number | (() => number);
    onHostIntent?: (intent: Record<string, unknown>, result: Record<string, unknown>) => unknown;
  },
): EngramMosaicHost;
