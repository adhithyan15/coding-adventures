import { makeController, type ControllerInit } from "./controller";
import {
  loadWorkspace,
  makeWorkspaceRecord,
  openWorkspaceStorage,
  preserveRejectedWorkspace,
  saveWorkspace,
  type WorkspaceStorageSession,
} from "./persistence";
import {
  applyThemeGround,
  boardAccent,
  ringGradient,
  storeTheme,
  type Theme,
} from "./theme";
import { createTaskEngine } from "./task-engine.mjs";

const HOST_READY_EVENT = "mosaic-host-ready";

export interface MosaicHostRequest {
  component: string;
  event?: Record<string, unknown>;
}

export interface TaskAppWebComponentHost {
  getProps(request: MosaicHostRequest): { props: Record<string, unknown> };
  handleEvent(request: MosaicHostRequest): { props: Record<string, unknown> };
}

interface HostOptions {
  controller: ReturnType<typeof makeController>;
  storageSession: Pick<WorkspaceStorageSession, "status" | "location" | "warning">;
  readStorageWarning?: () => string;
  theme: Theme;
  onThemeChange?: (theme: Theme) => void;
}

interface BootOptions {
  fetchWasm?: () => Promise<ArrayBuffer>;
  openStorage?: typeof openWorkspaceStorage;
  theme?: Theme;
  onThemeChange?: (theme: Theme) => void;
  now?: () => number;
}

declare global {
  interface Window {
    mosaicHost?: {
      getProps?: (request: MosaicHostRequest) => unknown | Promise<unknown>;
      handleEvent?: (request: MosaicHostRequest) => unknown | Promise<unknown>;
    };
  }
}

function themedProps(
  controller: ReturnType<typeof makeController>,
  storageSession: Pick<WorkspaceStorageSession, "status" | "location" | "warning">,
  storageWarning: string,
  theme: Theme,
): Record<string, unknown> {
  const props = controller.getProps();
  return {
    ...props,
    storageStatus: storageSession.status,
    storageLocation: storageSession.location,
    storageWarning,
    themeIsDark: theme === "dark" ? "dark" : "",
    ringGradient: ringGradient(theme, props.ringPercentValue),
    boardColumns: props.boardColumns.map((row: string[]) => [
      row[0],
      row[1],
      row[2],
      boardAccent(theme, row[1]),
    ]),
  };
}

/**
 * Adapt the framework-neutral TaskApp controller to the standard browser
 * `window.mosaicHost` protocol consumed by the generated Custom Element shell.
 */
export function createTaskAppWebComponentHost(options: HostOptions): TaskAppWebComponentHost {
  let theme = options.theme;
  let eventWarning = "";

  const response = () => ({
    props: themedProps(
      options.controller,
      options.storageSession,
      eventWarning || options.readStorageWarning?.() || options.storageSession.warning,
      theme,
    ),
  });

  return {
    getProps() {
      return response();
    },
    handleEvent(request) {
      const event = request.event;
      if (!event || typeof event.type !== "string") {
        return response();
      }
      if (event.type === "toggleTheme") {
        theme = theme === "dark" ? "light" : "dark";
        storeTheme(theme);
        options.onThemeChange?.(theme);
        return response();
      }
      try {
        options.controller.apply(event as never);
      } catch (error) {
        const detail = error instanceof Error ? error.message : String(error);
        eventWarning = `Trestle could not apply that change: ${detail}`;
      }
      return response();
    },
  };
}

function pageTheme(): Theme {
  return document.documentElement.dataset.theme === "dark" ? "dark" : "light";
}

function navigateToTheme(theme: Theme): void {
  const next = new URL(window.location.href);
  next.pathname = next.pathname.replace(/\/(?:light|dark)(?:\.html)?$/, `/${theme}.html`);
  window.location.assign(next);
}

async function defaultFetchWasm(): Promise<ArrayBuffer> {
  // The parity pages and public WASM file share the bundle root. Keep this
  // relative so the whole gate remains relocatable under any CI directory.
  const response = await fetch(new URL("./task_engine.wasm", window.location.href));
  if (!response.ok) {
    throw new Error(
      `The scheduling engine could not be downloaded (HTTP ${response.status} ${response.statusText}).`,
    );
  }
  return response.arrayBuffer();
}

async function initializeHost(options: BootOptions): Promise<TaskAppWebComponentHost> {
  const engine = createTaskEngine(await (options.fetchWasm ?? defaultFetchWasm)());
  const storageSession = await (options.openStorage ?? openWorkspaceStorage)();
  const storage = storageSession.storage;
  let warning = storageSession.warning;
  let saved: Awaited<ReturnType<typeof loadWorkspace>>;

  try {
    saved = await loadWorkspace(storage);
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    warning = `Saved data could not be read. Trestle started a fresh workspace: ${detail}`;
    saved = undefined;
  }

  if (saved) {
    try {
      engine.load(saved.snapshot);
      if (saved.activeProject) {
        engine.setActiveProject({ id: saved.activeProject });
      }
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      try {
        await preserveRejectedWorkspace(storage, saved);
        warning =
          "Saved data could not be restored. Trestle started a fresh workspace and kept " +
          `the rejected record as workspace/web-corrupt for recovery: ${detail}`;
      } catch (preserveError) {
        const preserveDetail =
          preserveError instanceof Error ? preserveError.message : String(preserveError);
        warning =
          "Saved data could not be restored, and its recovery copy could not be written. " +
          `Do not clear this site's browser data: ${detail}; ${preserveDetail}`;
      }
      saved = undefined;
    }
  }

  const session = { ...storageSession, warning };
  const controllerOptions: ControllerInit = {
    initialOrder: saved?.order ?? [],
    initialCounter: saved?.counter ?? 0,
    now: options.now,
    onMutate: (snapshot, order, counter, activeProject) =>
      saveWorkspace(
        storage,
        makeWorkspaceRecord(snapshot, order, counter, (options.now ?? Date.now)(), activeProject),
        (message) => {
          warning = message;
        },
      ),
  };
  const controller = makeController(engine, controllerOptions);
  return createTaskAppWebComponentHost({
    controller,
    storageSession: session,
    readStorageWarning: () => warning,
    theme: options.theme ?? pageTheme(),
    onThemeChange: options.onThemeChange ?? navigateToTheme,
  });
}

function setStartupState(state: "loading" | "ready" | "failed", detail = ""): void {
  document.body.dataset.webcomponentState = state;
  const status = document.getElementById("startup-status");
  if (status) {
    status.textContent =
      state === "loading"
        ? "Starting Trestle…"
        : state === "failed"
          ? `Trestle could not start. ${detail}`
          : "";
  }
}

/** Install the asynchronous host facade before the emitted runtime asks for props. */
export function bootWebComponentHost(options: BootOptions = {}): Promise<void> {
  const theme = options.theme ?? pageTheme();
  applyThemeGround(theme);
  setStartupState("loading");

  const ready = initializeHost({ ...options, theme });
  window.mosaicHost = {
    async getProps(request) {
      return (await ready).getProps(request);
    },
    async handleEvent(request) {
      return (await ready).handleEvent(request);
    },
  };

  return ready.then(
    () => {
      setStartupState("ready");
      window.dispatchEvent(new CustomEvent(HOST_READY_EVENT));
    },
    (error: unknown) => {
      const detail = error instanceof Error ? error.message : String(error);
      setStartupState("failed", detail);
    },
  );
}

if (document.querySelector("mos-task-app")) {
  void bootWebComponentHost();
}
