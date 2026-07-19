// Dependency-free JavaScript accessor for the task-wasm ABI.
//
// It surfaces the pure `task-core` engine to JavaScript — **one method per engine
// operation/query**. There is no dispatch bus and no props/events facade: a web/React
// host keeps this engine object in state and calls its methods directly, re-rendering
// on change. That is idiomatic React; the React-isms live here, not in the engine.
//
//   const engine = createTaskEngine(wasmBytes);
//   engine.createTask({ id: "a", name: "Write spec" });   // → { ok: true }
//   engine.checklist();                                    // → { ok: true, data: [...] }
//   engine.gantt(projectStartDays);                        // → { ok: true, data: {...} }
//   const json = engine.snapshot();  engine.load(json);    // persistence

export function createTaskEngine(wasmBytes, options = {}) {
  const module =
    wasmBytes instanceof WebAssembly.Module ? wasmBytes : new WebAssembly.Module(wasmBytes);
  const instance = new WebAssembly.Instance(module, options.importObject ?? defaultImports());
  const ex = instance.exports;
  const enc = new TextEncoder();
  const dec = new TextDecoder();
  const mem = () => new Uint8Array(ex.memory.buffer);

  function writeStr(value) {
    const bytes = enc.encode(String(value));
    if (bytes.length === 0) return [0, 0];
    const ptr = ex.alloc(bytes.length);
    if (ptr === 0) throw new Error("task-wasm alloc returned null");
    mem().set(bytes, ptr);
    return [ptr, bytes.length];
  }

  function readResult(ptr) {
    if (ptr === 0) throw new Error("task-wasm returned null");
    const m = mem();
    const len =
      (m[ptr] | (m[ptr + 1] << 8) | (m[ptr + 2] << 16) | (m[ptr + 3] << 24)) >>> 0;
    const value = dec.decode(m.subarray(ptr + 4, ptr + 4 + len));
    ex.dealloc(ptr, 4 + len);
    return value;
  }

  // Call an export that takes a `(ptr,len)` string and returns a JSON envelope.
  function callStr(name, str) {
    const [ptr, len] = writeStr(str);
    const out = ex[name](ptr, len);
    if (len) ex.dealloc(ptr, len);
    return JSON.parse(readResult(out));
  }
  // Call a JSON-payload operation.
  const op = (name) => (payload) => callStr(name, JSON.stringify(payload ?? {}));
  // Call a no-argument query.
  const query = (name) => () => JSON.parse(readResult(ex[name]()));

  return {
    // ── lifecycle ──
    reset() {
      ex.reset();
    },
    /** Serialize the whole workspace (raw JSON string) for host-owned persistence. */
    snapshot() {
      return readResult(ex.snapshot());
    },
    /**
     * Replace the workspace with a snapshot JSON string. Accepts either a whole
     * workspace snapshot or a pre-workspace bare-project snapshot (migrated on load),
     * so data persisted before workspaces keeps loading.
     */
    load(json) {
      return callStr("load", json);
    },

    // ── operations (validated; each returns { ok } or { ok:false, error, code }) ──
    createTask: op("create_task"),
    renameTask: op("rename_task"),
    deleteTask: op("delete_task"),
    reparent: op("reparent"),
    setKind: op("set_kind"),
    setCompleted: op("set_completed"),
    setPercentComplete: op("set_percent_complete"),
    setStatus: op("set_status"),
    setSchedule: op("set_schedule"),
    setDuration: op("set_duration"),
    setConstraint: op("set_constraint"),
    setDeadline: op("set_deadline"),
    linkDependency: op("link_dependency"),
    unlinkDependency: op("unlink_dependency"),
    addLink: op("add_link"),
    upsertResource: op("upsert_resource"),
    assign: op("assign"),
    upsertField: op("upsert_field"),
    setFieldValue: op("set_field_value"),
    setDecision: op("set_decision"),
    answerDecision: op("answer_decision"),
    setProjectName: op("set_project_name"),

    // ── workspace operations (across projects) ──
    createProject: op("create_project"),
    renameProject: op("rename_project"),
    deleteProject: op("delete_project"),
    nestProject: op("nest_project"),
    unnestProject: op("unnest_project"),
    /** Create a task in a named project (workspace-global id uniqueness). */
    createTaskIn: op("create_task_in"),
    moveTask: op("move_task"),
    linkCrossProjectDependency: op("link_cross_project_dependency"),
    unlinkCrossProjectDependency: op("unlink_cross_project_dependency"),
    upsertSharedResource: op("upsert_shared_resource"),
    deleteSharedResource: op("delete_shared_resource"),

    // ── queries / projections (each returns { ok:true, data }) ──
    checklist: query("checklist"),
    todos: query("todos"),
    flowchart: query("flowchart"),
    /** The whole workspace: projects, nesting, cross-project edges, shared pool. */
    workspace: query("workspace"),
    /** Whole-workspace CPM schedule anchored at a project start (days since epoch). */
    workspaceSchedule(projectStartDays) {
      return JSON.parse(readResult(ex.workspace_schedule(projectStartDays | 0)));
    },
    /** Gantt timeline anchored at a project start (days since the Unix epoch). */
    gantt(projectStartDays) {
      return JSON.parse(readResult(ex.gantt(projectStartDays | 0)));
    },
    /** CPM schedule anchored at a project start (days since the Unix epoch). */
    schedule(projectStartDays) {
      return JSON.parse(readResult(ex.schedule(projectStartDays | 0)));
    },
    /** Kanban board for the workflow with the given id. */
    kanban(workflowId) {
      return callStr("kanban", String(workflowId));
    },
  };
}

// The ABI needs no host imports; provide an empty object plus a tiny env in case a
// toolchain expects one.
function defaultImports() {
  return { env: {} };
}
