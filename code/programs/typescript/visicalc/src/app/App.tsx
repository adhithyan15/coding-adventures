import { useEffect, useRef, useState, type ComponentProps, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { VisiCalc as Light } from "../components/light/react/VisiCalc";
import { VisiCalc as Dark, type VisiCalcEvent } from "../components/dark/react/VisiCalc";
import { VisiCalcStartup as StartupLight } from "../components/light/react/VisiCalcStartup";
import { VisiCalcStartup as StartupDark } from "../components/dark/react/VisiCalcStartup";
import { loadMosaicModule, type MosaicHost, type MosaicUpdate } from "../../../../../packages/rust/mosaic-app-wasm/js/mosaic-host.mjs";
import { createBrowserFileEffects, type MosaicFileEffects } from "../../../../../packages/rust/mosaic-app-wasm/js/mosaic-file-effects.mjs";

export async function loadApplication(): Promise<MosaicHost> {
  const response = await fetch("/visicalc_mosaic_app.wasm");
  if (!response.ok) throw new Error(`Could not load VisiCalc (${response.status})`);
  const module = await loadMosaicModule(await response.arrayBuffer());
  return module.create({ protocolVersion: 2, colorScheme: window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light" });
}

// The host translates browser input into semantic events; Rust owns all state.
export function App({ load = loadApplication }: { load?: () => Promise<MosaicHost> }) {
  const host = useRef<MosaicHost | null>(null);
  const files = useRef<MosaicFileEffects | null>(null);
  const surface = useRef<HTMLDivElement | null>(null);
  const [update, setUpdate] = useState<MosaicUpdate | null>(null);
  const [error, setError] = useState("");
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let live = true;
    let owned: MosaicHost | null = null;
    setError("");
    setUpdate(null);
    Promise.resolve().then(load).then(app => {
      owned = app;
      if (!live) { app.dispose(); return; }
      host.current = app;
      files.current = app.update.protocolVersion === 2 ? createBrowserFileEffects(app) : null;
      setUpdate(app.update);
    }).catch(reason => { if (live) setError(String(reason)); });
    return () => { live = false; files.current?.dispose(); files.current = null; owned?.dispose(); host.current = null; };
  }, [load, attempt]);
  const started = update !== null;
  useEffect(() => {
    if (started && attempt > 0) surface.current?.querySelector<HTMLTableElement>('table[tabindex="0"]')?.focus();
  }, [started, attempt]);
  const render = (next: MosaicUpdate, app: MosaicHost) => {
    if (host.current !== app) return;
    setUpdate(next);
    for (const effect of next.effects) {
      // Start the picker now, within dispatch's originating user gesture.
      // Waiting for a React effect would lose browser activation.
      void files.current?.run(effect).then(completed => {
        if (completed && host.current === app) render(completed, app);
      }).catch(reason => { if (host.current === app) setError(String(reason)); });
    }
  };
  const send = (name: string, payload: Record<string, unknown> = {}) => {
    if (!host.current) return;
    try { render(host.current.dispatch(name, payload), host.current); setError(""); }
    catch (reason) { setError(String(reason)); }
  };
  const key = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.defaultPrevented || !host.current) return;
    if (!(event.target instanceof HTMLElement) || !event.target.closest('table[tabindex="0"]')) return;
    if (event.altKey || event.ctrlKey || event.metaKey || event.nativeEvent.isComposing) return;
    if (event.target instanceof HTMLElement &&
        (event.target.closest("input, textarea, select, button") || event.target.isContentEditable)) return;
    const p = host.current.update.props;
    if (p.editing) return;
    const row = Number(p["selected-row"]), col = Number(p["selected-col"]);
    const delta: Record<string, [number, number]> = { ArrowUp: [-1, 0], ArrowDown: [1, 0], ArrowLeft: [0, -1], ArrowRight: [0, 1] };
    if (event.shiftKey && (delta[event.key] || event.key === "Home" || event.key === "End")) return;
    if (delta[event.key]) {
      const [dr, dc] = delta[event.key];
      send("navigate", { row: Math.max(0, Math.min(Number(p["total-rows"]) - 1, row + dr)), col: Math.max(0, Math.min(Number(p["total-cols"]) - 1, col + dc)) });
      event.preventDefault();
    } else if (event.key === "Home" || event.key === "End") {
      send("navigate", { row, col: event.key === "Home" ? 0 : Number(p["total-cols"]) - 1 });
      event.preventDefault();
    } else if (event.key === "Enter" || event.key === "F2") {
      send("editStart", { row, col }); event.preventDefault();
    } else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      send("editStart", { row, col }); send("formulaChange", { value: event.key }); event.preventDefault();
    }
  };
  if (!update) {
    const Startup = window.matchMedia?.("(prefers-color-scheme: dark)").matches ? StartupDark : StartupLight;
    return <section role={error ? "alert" : "status"}>
      <Startup heading={error ? "Your workbook couldn’t open" : "Opening your workbook"}
        message={error ? "VisiCalc couldn’t start. Check your connection and try again." : "Making space for your numbers, notes and next big idea."}
        canRetry={!!error} dispatch={() => setAttempt(value => value + 1)} />
    </section>;
  }
  const props = Object.fromEntries(Object.entries(update.props).map(([name, value]) => [name.replace(/-([a-z])/g, (_, letter: string) => letter.toUpperCase()), value])) as Omit<ComponentProps<typeof Light>, "dispatch">;
  const View = update.props["dark-theme"] ? Dark : Light;
  const dispatch = ({ type, ...payload }: VisiCalcEvent) => send(type, payload);
  return <div ref={surface} onKeyDown={key}>
    <View {...props} dispatch={dispatch} />
    {error && <div role="alert">{error}</div>}
    <div role="status" aria-live="polite" aria-atomic="true" style={{ position: "absolute", top: 0, left: 0, width: 1, height: 1, overflow: "hidden", clipPath: "inset(50%)", whiteSpace: "nowrap" }}>{update.announcements.map(item => item.message).join(". ")}</div>
  </div>;
}
