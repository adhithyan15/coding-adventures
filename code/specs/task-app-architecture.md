# task-app — Application Architecture

> Part of the [task-app spec series](task-app-overview.md). Defines the crate layering and the
> engine ↔ UI seam.
>
> **This spec was rewritten (2026-07-11) to remove imported "Reactism".** The original draft copied
> Engram's Flux command/dispatch reducer and a single `dispatch`/`getProps`/`handleEvent` JSON facade
> and imposed that React-shaped contract on all nine platforms. That is wrong: SwiftUI, Compose, Qt,
> Flutter, and WinUI do not work like React, and forcing a props/events dispatch model onto them
> produces awkward, non-idiomatic code. The corrected architecture is below.

## Two principles

1. **The engine is pure computation only.** `task-core` is the data model plus pure functions —
   validated operations, queries/projections, the CPM scheduler, calendar math, and formula
   evaluation. It has no I/O, no clock (`now: u64` is passed in), and **no state-management runtime**
   (no store, no command bus, no dispatch loop). It is a library of pure functions over a value.

2. **Each backend adopts its own native conventions.** There is **no universal facade**. Every host
   holds the engine's state in *its platform's* native container and calls the engine's functions
   directly, observing changes the platform's own way. React-isms live only in the React (web)
   backend — never leaked into the others.

## The layer cake

```
task-core            THE PURE ENGINE (no I/O, no clock, no state runtime, forbid(unsafe)):
                       • model types (ProjectState and its entities)
                       • pure operations API — validated &mut methods (the trust boundary)
                       • pure queries / projections — checklist, todo, kanban, gantt,
                         flowchart, table, computed fields
                       • scheduler (CPM), calendar (working time), formula (symbolic-vm)
  ├ task-capi        C ABI exposing the engine's functions to native shells (opaque handle)
  └ task-wasm        linear-memory WASM ABI exposing the same to browser / Electron (+ JS accessor)

per-backend NATIVE host bindings (each idiomatic — NOT a shared facade):
  web / React        engine (via task-wasm) held in React state / hooks   — React-isms belong here
  SwiftUI            @Observable model wraps the engine (via task-capi)
  Jetpack Compose    State / ViewModel wraps the engine (via task-capi + JNA)
  Qt                 QAbstractItemModel + signals/slots over the engine (via task-capi)
  Flutter            ChangeNotifier / Riverpod over the engine (via task-capi + Dart FFI)
  WinUI / XAML       observable dependency properties over the engine (via task-capi)
```

`task-capi` and `task-wasm` are thin ABIs over the **same pure engine functions** — they add
marshalling only, never behavior or state. Only they contain `unsafe`; everything in `task-core` is
`forbid(unsafe_code)`.

## `task-core` — the pure engine API

### Operations (validated mutations)

Mutations are ordinary methods on `ProjectState`, returning `Result<_, OpError>`. They mutate the
value in place (pure: deterministic, no I/O, no globals) and are the **single trust boundary** that
enforces the model's invariants — the validation formerly attributed to a "reducer" lives here, with
no command enum and no dispatch:

```rust
pub enum OpError { NotFound, Duplicate, WouldCycle, Invalid(&'static str) }

impl ProjectState {
    pub fn create_task(&mut self, id: TaskId, name: impl Into<String>, parent: Option<TaskId>) -> Result<(), OpError>;
    pub fn rename_task(&mut self, id: &TaskId, name: impl Into<String>) -> Result<(), OpError>;
    pub fn delete_task(&mut self, id: &TaskId) -> Result<(), OpError>;   // reparents children, prunes links
    pub fn reparent(&mut self, id: &TaskId, new_parent: Option<TaskId>) -> Result<(), OpError>; // cycle-checked
    pub fn set_percent_complete(&mut self, id: &TaskId, pct: u8) -> Result<(), OpError>;        // clamped 0..=100
    pub fn set_schedule(&mut self, id: &TaskId, schedule: Option<TaskSchedule>) -> Result<(), OpError>;
    pub fn link_dependency(&mut self, link: DependencyLink) -> Result<(), OpError>; // self/dup/cycle-checked
    pub fn upsert_resource(&mut self, r: Resource) -> Result<(), OpError>;
    pub fn assign(&mut self, a: Assignment) -> Result<(), OpError>;
    pub fn add_calendar_exception(&mut self, cal: &CalendarId, ex: CalendarException) -> Result<(), OpError>; // interval-validated
    pub fn set_field_value(&mut self, task: &TaskId, field: &FieldId, value: Option<FieldValue>) -> Result<(), OpError>;
    pub fn answer_decision(&mut self, id: &TaskId, answer: bool) -> Result<(), OpError>;
    pub fn capture_baseline(&mut self, id: BaselineId, name: String, now: u64) -> Result<(), OpError>;
    // … one method per operation; each returns Ok or a typed rejection.
}
```

A backend that prefers value semantics simply `clone()`s first — the methods do not impose
immutability, and they do not impose a command/dispatch shape.

### Queries / projections (read-only)

Every view primitive is a pure function over the state — this is the "one model, many views" thesis
made concrete, and the "rock-solid API across all the primitives" the app is built on:

```rust
impl ProjectState {
    pub fn schedule(&self, project_start: Date) -> Result<ScheduleResult, SchedulingError>; // CPM
    pub fn checklist(&self, view: &View) -> Vec<ChecklistRow>;   // flatten decision-visible items
    pub fn todos(&self, view: &View) -> Vec<TodoRow>;
    pub fn kanban(&self, view: &View) -> Vec<KanbanColumn>;
    pub fn gantt(&self, project_start: Date) -> GanttView;       // bars from schedule()
    pub fn flowchart(&self) -> FlowGraph;                        // tasks as nodes, links as edges
    pub fn table(&self, view: &View) -> TableData;
    pub fn computed_fields(&self) -> BTreeMap<TaskId, BTreeMap<FieldId, FieldValue>>; // formula/rollup
}
```

### Serialization

`serde` (behind the feature) gives `to_json` / `from_json` for `ProjectState`. Persistence is
**host-owned**: the host serialises the value and writes it wherever is native (localStorage, a file,
a keychain) and hands bytes back to `from_json`. The engine performs no I/O.

## `task-capi` — C ABI (native shells)

An opaque handle over a `ProjectState`, with functions that mirror the engine API 1:1. No
"session", no "dispatch", no props/events:

```c
ProjectState* ts_new(void);
void          ts_free(ProjectState*);
// operations: typed args or a small JSON arg; return 0 or an OpError code
int32_t       ts_create_task(ProjectState*, const char* id, const char* name, const char* parent_or_null);
int32_t       ts_link_dependency(ProjectState*, const char* link_json);
// queries: return owned JSON strings (freed by ts_string_free)
char*         ts_schedule(ProjectState*, int32_t project_start_days);
char*         ts_checklist(ProjectState*, const char* view_json);
char*         ts_gantt(ProjectState*, int32_t project_start_days);
char*         ts_snapshot(ProjectState*);
int32_t       ts_load(ProjectState*, const char* json);
void          ts_string_free(char*);
```

FFI input enums are primitive ints validated via `TryFrom` (never `repr(C)` Rust enums). Binary
import/export uses `(ptr, len)` byte pairs. A native host calls these from *its* model layer.

## `task-wasm` — WASM ABI (browser / Electron)

The same engine functions over a hand-rolled linear-memory ABI (`alloc`/`dealloc`, `(ptr,len)` in,
length-prefixed out), plus a small JS accessor:

```ts
const engine = await createTaskEngine(wasmBytes);   // holds one ProjectState
engine.createTask(id, name, parent);                // → throws on OpError
engine.schedule(projectStart);                       // → ScheduleResult
engine.checklist(view); engine.gantt(projectStart);  // → view data
engine.snapshot(); engine.load(json);
```

The web/React host keeps this engine in React state and re-renders on change — idiomatic React, not
a `getProps`/`handleEvent` bus.

## Mosaic UI + native wiring

The UI is authored **once in Mosaic** (`.mil`/`.mll`/`.msl`) and emitted per backend. But the *data
in* and *actions out* are wired **natively**, not through a universal contract:

- **Web/React**: the Mosaic-emitted React components receive data as **props from React state**
  (backed by the wasm engine) and raise callbacks that call engine methods.
- **SwiftUI**: the Mosaic-emitted views bind to an **`@Observable`** model that wraps `task-capi`.
- **Compose**: bind to Compose **`State`** exposed by a ViewModel over `task-capi`.
- **Qt**: back the Mosaic views with a **`QAbstractItemModel`**; actions are slots calling `task-capi`.

Reusable `code/packages/mosaic/mosaic-pkg-*` components cover the primitives (checklist-runner,
todo-list, task-board, gantt-view, flowchart-view, task-detail, field-editor). Mosaic is the view
layer; each backend owns its state and reactivity.

## What this deliberately does NOT have

- No `reduce(state, command)` / `TaskCommand` enum / command bus (a Flux/React idiom).
- No `task-core-wasm` "session" facade, no `dispatch` / `getProps` / `handleEvent` universal contract.
- No kebab-case slot map treated as the cross-platform wire format.

The engine is functions over a value; the platforms are native. That is the whole design.

## Testing

- **Engine**: exhaustive pure unit tests (operations incl. validation/rejection; every projection;
  scheduler correctness; formula eval) — no mocks needed, since there is no I/O.
- **ABIs**: round-trip tests that the C/WASM boundary faithfully forwards to the engine.
- **Host bindings**: tested in each backend's native idiom (SwiftUI previews, Compose tests, etc.).
- The "contract" is the engine's typed Rust API, not a JSON props schema.
