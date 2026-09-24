// App.tsx — the Journal web host (J5c-2; spec journal-mosaic-app.md).
//
// A thin host, in VisiCalc's shape. The Rust runtime (journal-mosaic-app,
// built for wasm32) owns every piece of state; the generated JournalApp draws
// its props. This file only:
//
//   * loads the wasm, giving it the clock it imports (journal.now_ms);
//   * passes props through (kebab-case keys become camelCase);
//   * sends every event straight to the runtime, then files the snapshot away;
//   * restores the last snapshot on boot.
import { useEffect, useRef, useState, type ComponentProps } from "react";
import { JournalApp as Light, type JournalAppEvent } from "./components/light/react/JournalApp";
import { JournalApp as Dark } from "./components/dark/react/JournalApp";
import { loadMosaicModule, type MosaicHost, type MosaicUpdate } from "../../../../../../packages/rust/mosaic-app-wasm/js/mosaic-host.mjs";
import { browserStorage, readStored, setAside, STATE_KEY, writeSnapshot } from "./persistence";

function prefersDark(): boolean {
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false;
}

/**
 * The clock the runtime imports. It must RETURN: an exception thrown out of a
 * wasm import would unwind past the runtime's Rust frames. So anything it
 * throws becomes NaN, which the runtime reads as the epoch.
 */
export function hostClock(read: () => number = Date.now): () => number {
  return () => {
    try {
      return Number(read());
    } catch {
      return Number.NaN;
    }
  };
}

/**
 * The browser's UTC offset in minutes EAST of UTC, for StartContext's
 * utcOffsetMinutes (UI38 "Local time"): Journal files an entry under the
 * user's local day, not Greenwich's. `getTimezoneOffset()` counts minutes
 * WEST, hence the minus. The runtime refuses anything outside -840..=840 and
 * would not start, so a value that is not a plausible offset is left out
 * instead, and Journal falls back to UTC.
 */
export function browserUtcOffsetMinutes(date: Date = new Date()): number | undefined {
  try {
    // `0 - x`, not `-x`: UTC must read as 0, not -0.
    const east = 0 - date.getTimezoneOffset();
    return Number.isInteger(east) && east >= -840 && east <= 840 ? east : undefined;
  } catch {
    return undefined;
  }
}

export async function loadRuntime(
  bytes: BufferSource,
  dark = prefersDark(),
  { now = Date.now, utcOffsetMinutes = browserUtcOffsetMinutes() }: { now?: () => number; utcOffsetMinutes?: number } = {},
): Promise<MosaicHost> {
  const module = await loadMosaicModule(bytes, { journal: { now_ms: hostClock(now) } });
  return module.create({
    protocolVersion: 2,
    colorScheme: dark ? "dark" : "light",
    // Omitted when unknown, so the runtime falls back to UTC.
    ...(utcOffsetMinutes === undefined ? {} : { utcOffsetMinutes }),
  });
}

export async function loadApplication(): Promise<MosaicHost> {
  const response = await fetch("/journal_mosaic_app.wasm");
  if (!response.ok) throw new Error(`Could not load Journal (${response.status})`);
  return loadRuntime(await response.arrayBuffer());
}

type Props = Omit<ComponentProps<typeof Light>, "dispatch">;

function camelProps(props: Record<string, unknown>): Props {
  return Object.fromEntries(
    Object.entries(props).map(([name, value]) => [name.replace(/-([a-z])/g, (_, letter: string) => letter.toUpperCase()), value]),
  ) as Props;
}

export function App({
  load = loadApplication,
  storage = browserStorage(),
}: {
  load?: () => Promise<MosaicHost>;
  storage?: Storage | null;
}) {
  const host = useRef<MosaicHost | null>(null);
  const [update, setUpdate] = useState<MosaicUpdate | null>(null);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  // Set when another tab changes the stored journal. From then on this tab
  // stops saving: its snapshot is older, and writing it would silently undo
  // the other tab's work. Reloading picks the newer journal up.
  const stale = useRef(false);

  useEffect(() => {
    const changedElsewhere = (event: StorageEvent) => {
      // `storage` fires only for OTHER tabs' writes; null means cleared.
      if (event.key !== STATE_KEY && event.key !== null) return;
      stale.current = true;
      setNotice("Journal was changed in another tab. Reload this page to continue; changes made here are no longer saved.");
    };
    window.addEventListener("storage", changedElsewhere);
    return () => window.removeEventListener("storage", changedElsewhere);
  }, []);

  useEffect(() => {
    let live = true;
    let owned: MosaicHost | null = null;
    load()
      .then(app => {
        owned = app;
        if (!live) {
          app.dispose();
          return;
        }
        host.current = app;
        let first = app.update;
        const { snapshot, raw } = readStored(storage);
        if (raw !== null) {
          try {
            if (snapshot === null) throw new Error("not a Journal snapshot");
            first = app.restore(snapshot);
          } catch {
            // Never lose a journal: keep the value aside and start empty.
            setAside(storage, raw);
            setNotice("Your saved journal could not be opened. It has been kept aside, and a new journal started.");
          }
        }
        setUpdate(first);
      })
      .catch(reason => {
        if (live) setError(String(reason));
      });
    return () => {
      live = false;
      owned?.dispose();
      host.current = null;
    };
  }, [load, storage]);

  const persist = (app: MosaicHost) => {
    if (!storage || stale.current) return;
    try {
      const snapshot = app.snapshot();
      if (snapshot) writeSnapshot(storage, snapshot);
    } catch {
      setNotice("Journal could not save to this browser. Your changes are kept only until you close the page.");
    }
  };

  const dispatch = ({ type, ...payload }: JournalAppEvent) => {
    const app = host.current;
    if (!app) return;
    try {
      setUpdate(app.dispatch(type, payload));
      setError("");
    } catch (reason) {
      // The runtime refused the event and left its state as it was.
      setError(String(reason));
      return;
    }
    persist(app);
  };

  if (!update) {
    return <section role={error ? "alert" : "status"}>{error ? `Journal couldn't start: ${error}` : "Opening your journal…"}</section>;
  }
  const View = prefersDark() ? Dark : Light;
  return (
    <div>
      <View {...camelProps(update.props)} dispatch={dispatch} />
      {(error || notice) && <div role="alert">{error || notice}</div>}
      <div
        role="status"
        aria-live="polite"
        aria-atomic="true"
        style={{ position: "absolute", width: 1, height: 1, overflow: "hidden", clipPath: "inset(50%)", whiteSpace: "nowrap" }}
      >
        {update.announcements.map(item => item.message).join(". ")}
      </div>
    </div>
  );
}
