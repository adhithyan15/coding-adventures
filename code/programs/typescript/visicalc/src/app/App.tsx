import { useEffect, useRef, useState, type ComponentProps } from "react";
import { VisiCalc as Light } from "../components/light/react/VisiCalc";
import { VisiCalc as Dark, type VisiCalcEvent } from "../components/dark/react/VisiCalc";
import { loadMosaicModule, type MosaicHost, type MosaicUpdate } from "../../../../../packages/rust/mosaic-app-wasm/js/mosaic-host.mjs";

export async function loadApplication(): Promise<MosaicHost> {
  const response = await fetch("/visicalc_mosaic_app.wasm");
  if (!response.ok) throw new Error(`Could not load VisiCalc (${response.status})`);
  const module = await loadMosaicModule(await response.arrayBuffer());
  return module.create({ colorScheme: window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light" });
}

// The host translates browser input into semantic events; Rust owns all state.
export function App({ load = loadApplication }: { load?: () => Promise<MosaicHost> }) {
  const host = useRef<MosaicHost | null>(null);
  const [update, setUpdate] = useState<MosaicUpdate | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    let live = true;
    let owned: MosaicHost | null = null;
    load().then(app => {
      owned = app;
      if (!live) { app.dispose(); return; }
      host.current = app;
      setUpdate(app.update);
    }).catch(reason => { if (live) setError(String(reason)); });
    return () => { live = false; owned?.dispose(); host.current = null; };
  }, [load]);
  const send = (name: string, payload: Record<string, unknown> = {}) => {
    if (!host.current) return;
    try { setUpdate(host.current.dispatch(name, payload)); setError(""); }
    catch (reason) { setError(String(reason)); }
  };
  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      if (event.defaultPrevented || !host.current) return;
      if (event.target instanceof HTMLElement &&
          (event.target.closest("input, textarea, select, button") || event.target.isContentEditable)) return;
      const p = host.current.update.props;
      if (p.editing) return;
      const row = Number(p["selected-row"]), col = Number(p["selected-col"]);
      const delta: Record<string, [number, number]> = { ArrowUp: [-1, 0], ArrowDown: [1, 0], ArrowLeft: [0, -1], ArrowRight: [0, 1] };
      if (delta[event.key]) {
        const [dr, dc] = delta[event.key];
        send("navigate", { row: Math.max(0, Math.min(Number(p["total-rows"]) - 1, row + dr)), col: Math.max(0, Math.min(Number(p["total-cols"]) - 1, col + dc)) });
        event.preventDefault();
      } else if (event.key === "Enter" || event.key === "F2") {
        send("editStart", { row, col }); event.preventDefault();
      } else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
        send("editStart", { row, col }); send("formulaChange", { value: event.key }); event.preventDefault();
      }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, []);
  if (!update) return <div role="status">{error || "Opening workbook…"}</div>;
  const props = Object.fromEntries(Object.entries(update.props).map(([name, value]) => [name.replace(/-([a-z])/g, (_, letter: string) => letter.toUpperCase()), value])) as Omit<ComponentProps<typeof Light>, "dispatch">;
  const View = update.props["dark-theme"] ? Dark : Light;
  const dispatch = ({ type, ...payload }: VisiCalcEvent) => send(type, payload);
  return <>
    <View {...props} dispatch={dispatch} />
    {error && <div role="alert">{error}</div>}
    <div aria-live="polite">{update.announcements.map(item => item.message).join(". ")}</div>
  </>;
}
