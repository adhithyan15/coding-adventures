// Web host entry — wires the Mosaic-emitted TaskApp component to the pure task-core
// engine (via task-wasm's createTaskEngine) using plain React state. No window.mosaicHost
// facade: the component only needs {...slotProps, dispatch}. React-isms live here, in the
// web backend, where they belong.
import { StrictMode, useCallback, useState } from "react";
import { createRoot } from "react-dom/client";
import { TaskApp, type TaskAppEvent } from "../TaskApp";
import { createTaskEngine } from "./task-engine.mjs";

const DAY_MS = 86_400_000;
const isoToDays = (iso: string): number | null => {
  const m = /^\s*(\d{4})-(\d{2})-(\d{2})\s*$/.exec(iso);
  return m ? Math.floor(Date.UTC(+m[1], +m[2] - 1, +m[3]) / DAY_MS) : null;
};
const daysToIso = (days: number): string =>
  new Date(days * DAY_MS).toISOString().slice(0, 10);

// The controller is the web backend's native state container: it holds transient UI
// state (the two input values, the display order) and turns TaskApp events into engine
// operations, then re-derives the slot props from engine queries.
function makeController(engine: any) {
  let newName = "";
  let newDue = "";
  const order: string[] = []; // task ids in creation order
  let counter = 0;
  const today = Math.floor(Date.now() / DAY_MS);

  const todosById = (): Record<string, any> => {
    const map: Record<string, any> = {};
    for (const t of engine.todos().data) map[t.task] = t;
    return map;
  };
  const visible = (): string[] => {
    const t = todosById();
    return order.filter((id) => t[id]);
  };

  return {
    getProps() {
      const todos = todosById();
      const g = engine.gantt(today).data;
      const bars: Record<string, any> = {};
      for (const b of g.bars) bars[b.task] = b;
      const ids = order.filter((id) => todos[id]);
      const rows = ids.map((id) => {
        const t = todos[id];
        const b = bars[id];
        const check = t.completed ? "✓" : "○"; // ✓ / ○
        const due = t.deadline != null ? ` · due ${daysToIso(t.deadline)}` : "";
        const sched = b ? ` · ${daysToIso(b.start)} → ${daysToIso(b.finish)}` : "";
        const overdue =
          t.deadline != null && b && b.finish > t.deadline && !t.completed
            ? " · ⚠ overdue"
            : "";
        return `${check} ${t.name}${due}${sched}${overdue}`;
      });
      const doneCount = ids.filter((id) => todos[id].completed).length;
      const finish = g.projectFinish != null ? daysToIso(g.projectFinish) : "—";
      return {
        appTitle: "Tasks — auto-scheduled",
        newTaskName: newName,
        newTaskDue: newDue,
        summary: `${ids.length} task(s) · ${doneCount} done · projected finish ${finish}`,
        taskRows: rows,
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
          break;
        }
        case "toggleTask": {
          const id = visible()[event.index];
          if (id) engine.setCompleted({ id, completed: !todosById()[id].completed });
          break;
        }
        case "deleteTask": {
          const id = visible()[event.index];
          if (id) {
            engine.deleteTask({ id });
            const at = order.indexOf(id);
            if (at >= 0) order.splice(at, 1);
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
  const controller = makeController(engine);

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
