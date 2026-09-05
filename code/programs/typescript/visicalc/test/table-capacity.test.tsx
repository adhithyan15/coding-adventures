import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { mosaic$tableCapacityRef } from "../../../../packages/rust/mosaic-emit-react/src/table_capacity";

let frame: HTMLDivElement, table: HTMLTableElement;
let height: number, pitch: number;
let pending: Map<number, FrameRequestCallback>;
let observers: { notify: () => void; disconnected: boolean }[];
function flush() { const work = [...pending.values()]; pending.clear(); work.forEach(fn => fn(0)); }
beforeEach(() => {
  height = 340; pitch = 34; pending = new Map(); observers = [];
  let id = 0;
  vi.stubGlobal("requestAnimationFrame", (fn: FrameRequestCallback) => { pending.set(++id, fn); return id; });
  vi.stubGlobal("cancelAnimationFrame", (key: number) => pending.delete(key));
  vi.stubGlobal("ResizeObserver", class {
    disconnected = false;
    constructor(public notify: () => void) { observers.push(this); }
    observe() {}
    disconnect() { this.disconnected = true; }
  });
  frame = document.createElement("div"); frame.style.overflow = "auto";
  // jsdom does not synthesize overflow longhands or layout geometry.
  frame.style.overflowX = "auto"; frame.style.overflowY = "auto";
  frame.innerHTML = '<table style="border-collapse:collapse"><thead style="position:sticky"><tr><th>A</th></tr></thead><tbody><tr><td>1</td></tr><tr><td>2</td></tr></tbody></table>';
  document.body.append(frame); table = frame.querySelector("table")!;
  Object.defineProperty(frame, "clientHeight", { get: () => height });
  table.tHead!.getBoundingClientRect = () => ({ height: 34 } as DOMRect);
  Array.from(table.tBodies[0].rows).forEach((row, i) => {
    row.getBoundingClientRect = () => ({ height: pitch, top: i * pitch } as DOMRect);
  });
});
afterEach(() => { frame.remove(); vi.unstubAllGlobals(); vi.restoreAllMocks(); });

it("coalesces capacity, survives callback rebinding, and reacts to frame and row changes", () => {
  const report = vi.fn(); const first = mosaic$tableCapacityRef(report);
  first(table); observers[0].notify(); observers[0].notify(); flush();
  expect(report.mock.calls).toEqual([[9]]);
  first(null); expect(observers[0].disconnected).toBe(true);
  const second = mosaic$tableCapacityRef(report); second(table); flush();
  expect(report).toHaveBeenCalledTimes(1);
  height = 510; observers[1].notify(); flush();
  expect(report).toHaveBeenLastCalledWith(14);
  pitch = 50; observers[1].notify(); flush();
  expect(report).toHaveBeenLastCalledWith(9);
  second(null);
});

it("cancels queued deliveries and ignores stale observer callbacks after unmount", () => {
  const report = vi.fn(), ref = mosaic$tableCapacityRef(report);
  ref(table); const stale = [...pending.values()][0]; ref(null);
  expect(pending.size).toBe(0); expect(observers[0].disconnected).toBe(true);
  stale(0); observers[0].notify(); flush(); expect(report).not.toHaveBeenCalled();
});

it("waits for measurable rows and diagnoses nonuniform geometry once", () => {
  const report = vi.fn(), warning = vi.spyOn(console, "warn").mockImplementation(() => {});
  const ref = mosaic$tableCapacityRef(report); height = 0; ref(table); flush();
  expect(report).not.toHaveBeenCalled();
  height = 340;
  table.tBodies[0].rows[1].getBoundingClientRect = () => ({ height: 60, top: 34 } as DOMRect);
  observers[0].notify(); flush(); observers[0].notify(); flush();
  expect(report).not.toHaveBeenCalled(); expect(warning).toHaveBeenCalledTimes(1);
  ref(null);
});
