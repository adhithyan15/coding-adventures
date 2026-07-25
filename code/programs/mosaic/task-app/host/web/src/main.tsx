// Web host entry — wires the Mosaic-emitted TaskApp component to the pure task-core
// engine (via task-wasm's createTaskEngine) using plain React state, and persists the
// workspace through the pluggable storage layer (IndexedDB, in-memory fallback). No
// window.mosaicHost facade: the component only needs {...slotProps, dispatch}. React-isms
// and persistence both live here, in the web backend, where they belong — the engine
// stays a pure computation.
import { StrictMode, useCallback, useState } from "react";
import { createRoot } from "react-dom/client";
import { TaskApp, type TaskAppEvent } from "./TaskApp";
import { createTaskEngine } from "./task-engine.mjs";
import {
  loadWorkspace,
  makeWorkspaceRecord,
  openWorkspaceStorage,
  saveWorkspace,
} from "./persistence";

const DAY_MS = 86_400_000;

// The view the task list renders: which columns, in which order, sorted by name.
// This is the entire "what to show" decision — the engine does filtering, sorting,
// grouping, and formatting from it. `visibleFields` order defines the cell order.
const TASK_VIEW = (projectStart: number) => ({
  view: {
    id: "tasks",
    name: "Tasks",
    shape: "table",
    filter: { statuses: [], completed: null, search: null },
    groupBy: null,
    sort: [{ field: { builtin: "name" }, ascending: true }],
    visibleFields: [
      { builtin: "completed" },
      { builtin: "name" },
      { builtin: "deadline" },
      { builtin: "start" },
      { builtin: "finish" },
      { builtin: "overdue" },
    ],
  },
  projectStart,
});
const isoToDays = (iso: string): number | null => {
  const m = /^\s*(\d{4})-(\d{2})-(\d{2})\s*$/.exec(iso);
  return m ? Math.floor(Date.UTC(+m[1], +m[2] - 1, +m[3]) / DAY_MS) : null;
};
const daysToIso = (days: number): string =>
  new Date(days * DAY_MS).toISOString().slice(0, 10);

// State the controller can be seeded with on boot (restored from storage).
interface ControllerInit {
  initialOrder?: string[];
  initialCounter?: number;
  // Called after every *structural* mutation with the data worth persisting.
  onMutate?: (snapshot: string, order: string[], counter: number) => void;
}

// The controller is the web backend's native state container: it holds transient UI
// state (the two input values, the display order) and turns TaskApp events into engine
// operations, then re-derives the slot props from engine queries. After any mutation
// that changes the project it calls onMutate so the host can persist.
function makeController(engine: any, init: ControllerInit = {}) {
  const { initialOrder = [], initialCounter = 0, onMutate } = init;
  let newName = "";
  let newDue = "";
  const order: string[] = [...initialOrder]; // task ids in creation order
  let counter = initialCounter;
  const today = Math.floor(Date.now() / DAY_MS);

  // Snapshot the engine + host state and hand it to the persistence sink. Called
  // only after structural mutations (add/toggle/delete) — never on keystrokes.
  const persist = () => onMutate?.(engine.snapshot(), order, counter);

  // Column order matches TASK_VIEW's visibleFields.
  const [DONE, NAME, DEADLINE, START, FINISH, OVERDUE] = [0, 1, 2, 3, 4, 5];

  /// The ONE source of truth for what's on screen: the engine's table selection,
  /// keyed by task and ordered by creation. Both rendering and click-index resolution
  /// must read this same list — deriving them from different queries (e.g. rows from
  /// `table()` but indices from `todos()`) silently desyncs the moment the two disagree
  /// about which tasks qualify (milestones, filters), making a click hit the wrong row.
  const rows = (): { byTask: Map<string, any>; ids: string[] } => {
    const cells = engine.table(TASK_VIEW(today)).data.groups.flatMap((g: any) =>
      g.rows.map((r: any) => ({
        task: r.task as string,
        display: r.cells.map((c: any) => c.display as string),
        value: r.cells.map((c: any) => c.value),
      })),
    );
    const byTask = new Map<string, any>(cells.map((c: any) => [c.task, c]));
    return { byTask, ids: order.filter((id) => byTask.has(id)) };
  };
  const visible = (): string[] => rows().ids;

  return {
    getProps() {
      // Ask the ENGINE for render-ready cells. The host no longer formats dates,
      // picks the ✓/○ glyph, or decides what "overdue" means — those all come back
      // already resolved and formatted, identically for every future host. Each row
      // is handed to the layout as a *list of cells* in the order the interface
      // documents — [ done-glyph, name, due, schedule, overdue ] — and the Mosaic
      // layout places each cell in its own styled element (toggle, name, chips).
      // Empty cells become empty strings, which the layout hides.
      const { byTask, ids } = rows();
      const taskRows: string[][] = ids.map((id) => {
        const c = byTask.get(id)!;
        const due = c.display[DEADLINE] ? `due ${c.display[DEADLINE]}` : "";
        const sched = c.display[START] ? `${c.display[START]} → ${c.display[FINISH]}` : "";
        const late = c.value[OVERDUE]?.value === true ? "⚠ overdue" : "";
        return [c.display[DONE], c.display[NAME], due, sched, late];
      });
      const doneCount = ids.filter((id) => byTask.get(id)!.value[DONE]?.value === true).length;
      const finish = engine.gantt(today).data.projectFinish;
      return {
        appTitle: "Tasks — auto-scheduled",
        newTaskName: newName,
        newTaskDue: newDue,
        summary: `${ids.length} task(s) · ${doneCount} done · projected finish ${
          finish != null ? daysToIso(finish) : "—"
        }`,
        taskRows,
      };
    },

    apply(event: TaskAppEvent) {
      switch (event.type) {
        case "newTaskNameChange":
          newName = event.value;
          break;
        case "newTaskDueChange":
          newDue = event.value;
          break;
        case "addTask": {
          const name = newName.trim();
          if (!name) break;
          const id = `t${++counter}`;
          engine.createTask({ id, name });
          // A default one working-day duration makes the task schedulable.
          engine.setDuration({ id, duration: { workingMinutes: 8 * 60, elapsed: false } });
          // Chain after the last task so the engine builds a work queue (each task
          // starts when the previous finishes) — that's the "auto-schedule".
          const vis = visible();
          const prev = vis[vis.length - 1];
          if (prev) {
            engine.linkDependency({
              id: `l${counter}`,
              predecessor: prev,
              successor: id,
              kind: "finishToStart",
              lag: { workingMinutes: 0, elapsed: false },
            });
          }
          const due = isoToDays(newDue);
          if (due != null) engine.setDeadline({ id, deadline: due });
          order.push(id);
          newName = "";
          newDue = "";
          persist();
          break;
        }
        case "toggleTask": {
          // Resolve the row AND its current state from the same selection the UI drew,
          // so the index and the completed flag can never disagree.
          const { byTask, ids } = rows();
          const id = ids[event.index];
          if (id) {
            const done = byTask.get(id)?.value[DONE]?.value === true;
            engine.setCompleted({ id, completed: !done });
            persist();
          }
          break;
        }
        case "deleteTask": {
          const id = visible()[event.index];
          if (id) {
            engine.deleteTask({ id });
            const at = order.indexOf(id);
            if (at >= 0) order.splice(at, 1);
            persist();
          }
          break;
        }
      }
    },
  };
}

async function boot() {
  const bytes = await fetch("/task_engine.wasm").then((r) => r.arrayBuffer());
  const wasmModule = await WebAssembly.compile(bytes);
  const engine = createTaskEngine(wasmModule);

  // Restore the persisted workspace before the first render (no loading spinner):
  // load the engine snapshot, then seed the controller with the saved host state.
  const storage = await openWorkspaceStorage();
  const saved = await loadWorkspace(storage);
  if (saved) {
    try {
      engine.load(saved.snapshot);
    } catch (err) {
      console.error("Could not restore the saved workspace; starting fresh.", err);
    }
  }

  const controller = makeController(engine, {
    initialOrder: saved?.order ?? [],
    initialCounter: saved?.counter ?? 0,
    onMutate: (snapshot, order, counter) =>
      saveWorkspace(storage, makeWorkspaceRecord(snapshot, order, counter, Date.now())),
  });

  function Root() {
    const [props, setProps] = useState(() => controller.getProps());
    const dispatch = useCallback((event: TaskAppEvent) => {
      controller.apply(event);
      setProps(controller.getProps());
    }, []);
    return <TaskApp {...props} dispatch={dispatch} />;
  }

  const el = document.getElementById("root");
  if (!el) throw new Error("#root not found");
  createRoot(el).render(
    <StrictMode>
      <Root />
    </StrictMode>,
  );
}

void boot();
