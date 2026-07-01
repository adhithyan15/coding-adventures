# UI33 — Dispatcher Pattern + Business Logic Cores

> **Status.** Draft. Gates the UI33-G-* (grammar), UI33-E-* (per-
> backend core emitters), and UI33-R-* (reference-core) implementation
> cycles.
>
> **Parent.** UI29 — Primitive Kernel + Userland Component Packages
> (`code/specs/UI29-primitive-kernel.md`); composes with UI32 — Cross-
> backend project shells
> (`code/specs/UI32-cross-backend-project-shells.md`).
>
> **Scope.** Introduces two new DSLs (`.core` for business logic
> cores, `.disp` for dispatcher binding) and the runtime architecture
> that connects them. After this spec lands, every Mosaic-emitted
> project has a three-layer split — UI emits events, a typed
> Dispatcher routes them, swappable Cores own state and reducer logic
> — and the whole class of "host forgot to wire X" bugs becomes a
> compile error.

---

## 1. Motivation — the bug class PR #4594 exposed

PR [#4594](https://github.com/adhithyan15/coding-adventures/pull/4594)
fixed three independent failures in the VisiCalc React demo's inline
cell editor:

| # | Symptom                                            | Root cause                                                                                                                                                       |
|---|----------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 1 | Typing in a cell did nothing.                      | The cell's `<input>` had `value={v}` but no `onChange`. React's controlled-input contract silently rejects every keystroke when value never updates.             |
| 2 | Enter committed `undefined` to the cell.           | `Grid.mil` declared `emit onEditCommit (value: text)`, but `mosaic-emit-react::emit_host_input_jsx` emits a void `dispatch({type: "editCommit"})`. The reducer wrote `action.value` (= `undefined`).                                                                                          |
| 3 | Per-cell mouse click did nothing.                  | `Box [cell]` had no `onClick: emit: onNavigate(row: r, col: c)`. The grammar can't express parameterized emit dispatch with loop-bound names today.              |

All three failures share the same structural cause: **the host has to
hand-wire the contract between three independent files** (the `.mil`
emit signature, the per-backend emitter's lowering, and the host
reducer's action handler) and the framework verifies nothing.

The cells *visually* rendered correctly on all seven backends because
the view layer was right. The functional contract was wrong. There is
no test in the world that catches "the visual demo shows a cell with
an input inside, and the input is typed-into but the value never
makes it into the cell" without somebody noticing.

This is not a VisiCalc bug. It is the architecture inviting the bug.

## 2. The three-layer architecture

```
┌────────────────────────────────────────────────────────────┐
│  Host App                                                  │  custom domain logic
│  (formula engine, save-to-disk, network sync, ...)         │  ← host owns ONLY this
├────────────────────────────────────────────────────────────┤
│  Business Logic Cores         e.g. mosaic-core-grid        │  state + reducer + defaults
│  (override-friendly)               mosaic-core-form        │  ← packaged, swappable
│                                    mosaic-core-list        │
│                                    mosaic-core-tree   etc. │
├────────────────────────────────────────────────────────────┤
│  Dispatcher                                                │  typed action routing
│  (Action ─→ handler binding)                               │  ← generated, no manual wiring
├────────────────────────────────────────────────────────────┤
│  Mosaic UI                                                 │  view + event emission only
│  (.mil/.mll/.msl components, kernel primitives)            │  ← NO business logic ever
└────────────────────────────────────────────────────────────┘
```

The invariants this architecture enforces:

1. **Mosaic UI never owns state and never expresses logic.** A `.mil`
   component declares slots (inputs) and emits (outputs). A `.mll`
   layout declares the visual tree and which kernel-primitive events
   feed which emit. A `.msl` style applies presentation. Nothing in
   any of the three files can update state, compute derived values,
   or short-circuit an emit. They are pure-view.
2. **The Dispatcher is the only thing that crosses the UI ↔ logic
   boundary.** It is generated from the `.mil` emit declarations on
   one side and the `.core` action declarations on the other. There
   is exactly one place where a component emit becomes a typed action,
   and the generator produces it. Hand-wiring it is forbidden.
3. **Cores own state and reducer logic but expose extension points.**
   A `.core` file declares state shape, action handlers (the reducer),
   and explicit `extension-point` hooks where the host customises
   behaviour. The core ships sensible defaults so a host that wires
   nothing still gets a working application.
4. **Host code only fills extension points.** The host language is
   the target backend's natural one (TS for React, Swift for
   SwiftUI, etc.). The host writes domain logic — formula evaluation,
   network calls, persistence — and never reducer plumbing.

The bug class from §1 cannot occur in this architecture because:

- Bug #1 is impossible: when the `.core` declares `slot edit-content
  : text`, the dispatcher generator wires every HostInput with `value:
  slot: edit-content` to dispatch `editContentChange { value }`
  automatically. The host writes nothing.
- Bug #2 is impossible: `editCommit` is declared in the `.core` with
  its reducer logic; the dispatcher binds the HostInput's Enter to
  it; there is no host reducer for `action.value` to be `undefined`
  in.
- Bug #3 is impossible: `Box [cell] (onClick: emit: onNavigate(row:
  r, col: c))` is one valid grammar surface, the parameterized-
  emit-dispatch one (UI33-G-2). The Dispatcher routes the resulting
  `navigate(row, col)` action to the Grid core's existing handler.

## 3. The `.core` DSL

A core declares **state**, **actions** (reducer cases), and
**extension points**. It does not declare views.

### 3.1 Syntactic sketch

```
// mosaic-core-grid/src/grid.core
core Grid {
  // --- State shape, with default values for initialisation -------
  state {
    cells          : map<text, text>
    selected-row   : number = 0
    selected-col   : number = 0
    edit-row       : number = -1
    edit-col       : number = -1
    edit-content   : text   = ""
    total-rows     : number = 100
    total-cols     : number = 26
  }

  // --- Actions (reducer cases) ------------------------------------
  //
  // Each non-trivial action delegates AT LEAST ONE decision to an
  // extension point so the host can change behaviour without
  // replacing the whole action.  Core authors should be LIBERAL about
  // declaring extension points — every "this is the default; a real
  // app might want something else" decision is one.

  action navigate ( row : number , col : number ) {
    // Extension: navigation policy.  Default clamps to grid bounds;
    // hosts that want wrap-around, frozen panes, or scroll-into-view
    // override `clamp-navigation` to compute a different (row, col).
    let target = ext clamp-navigation ( row , col , total-rows , total-cols ) -> ( number , number )

    // Cancel any in-flight edit when navigating elsewhere.
    when edit-row != -1 {
      ext on-edit-cancelled ( edit-content , edit-row , edit-col )    // notify host (analytics, autosave)
      edit-row     = -1
      edit-col     = -1
      edit-content = ""
    }
    selected-row = target . 0
    selected-col = target . 1

    ext on-selection-changed ( selected-row , selected-col )           // notify host
  }

  action editStart ( row : number , col : number ) {
    // Extension: gate editing.  Default lets every cell edit; hosts
    // that want read-only cells, permission checks, or row-locking
    // override `can-edit` to return false.
    let allowed = ext can-edit ( row , col ) -> bool
    when allowed {
      edit-row     = row
      edit-col     = col
      // Extension: seed the editor.  Default is the cell's stored
      // value; hosts may seed a formula template or last-typed value.
      edit-content = ext initial-edit-content ( row , col , cells [ cell-key ( row , col ) ] or "" ) -> text
    }
  }

  action formulaChange ( value : text ) {
    // Extension: validate-as-you-type.  Default accepts every
    // keystroke; hosts may sanitise, length-clamp, or reject.
    edit-content = ext on-formula-change ( value , edit-row , edit-col ) -> text
  }

  action editCommit {
    when edit-row != -1 {
      // Extension: the value-transform on commit.  This is the
      // "formula engine plug-in point" — host returns the evaluated
      // / sanitised / parsed value to store.  Default is raw
      // passthrough.
      let stored = ext on-commit ( edit-content , edit-row , edit-col ) -> text

      cells [ cell-key ( edit-row , edit-col ) ] = stored

      // Extension: post-commit cursor policy.  Default moves down
      // one row (Excel convention).  Hosts may move right (Numbers),
      // stay in place (some accessibility configs), or jump to a
      // named cell.
      let next = ext post-commit-cursor ( edit-row , edit-col , total-rows , total-cols ) -> ( number , number )
      selected-row = next . 0
      selected-col = next . 1

      edit-row     = -1
      edit-col     = -1
      edit-content = ""

      ext on-cell-committed ( stored , next . 0 , next . 1 )          // notify host (autosave, recalc)
    }
  }

  action editCancel {
    ext on-edit-cancelled ( edit-content , edit-row , edit-col )      // shared with navigate
    edit-row     = -1
    edit-col     = -1
    edit-content = ""
  }

  // --- Explicit extension points the host can override ------------

  extension-point clamp-navigation ( r : number , c : number , rows : number , cols : number )
    -> ( number , number )
  {
    default {
      // Default: clamp to [0, rows) × [0, cols)
      ( max ( 0 , min ( r , rows - 1 ) ) , max ( 0 , min ( c , cols - 1 ) ) )
    }
  }

  extension-point can-edit ( row : number , col : number ) -> bool {
    default { true }
  }

  extension-point initial-edit-content ( row : number , col : number , current : text ) -> text {
    default { current }
  }

  extension-point on-formula-change ( value : text , row : number , col : number ) -> text {
    default { value }                                                  // accept every keystroke
  }

  extension-point on-commit ( value : text , row : number , col : number ) -> text {
    default { value }                                                  // raw text → raw text
  }

  extension-point post-commit-cursor ( row : number , col : number , rows : number , cols : number )
    -> ( number , number )
  {
    default { ( min ( row + 1 , rows - 1 ) , col ) }                  // move down
  }

  // --- Void extension points (notifications; no return value) -----

  extension-point on-selection-changed ( row : number , col : number ) {
    default { }                                                        // no-op
  }

  extension-point on-cell-committed ( value : text , next-row : number , next-col : number ) {
    default { }                                                        // no-op
  }

  extension-point on-edit-cancelled ( draft : text , row : number , col : number ) {
    default { }                                                        // no-op
  }
}
```

### 3.2 Grammar primitives

The `.core` DSL needs the following grammar surface. Each row is its
own UI33-G-* PR; together they define the language.

| ID         | Construct                          | Notes                                                                                                           |
|------------|------------------------------------|-----------------------------------------------------------------------------------------------------------------|
| UI33-G-1   | `core NAME { ... }` top-level      | Mirrors `component` in `.mil`. One core per file.                                                               |
| UI33-G-2   | `state { slot ... }` block         | Slot declarations with `name : type = default`. Reuses `.mil` types: text, number, bool, list<T>, map<K,V>, tuples `(T1, T2)`. |
| UI33-G-3   | `action NAME ( params ) { body }`  | Reducer case. Params are typed.                                                                                 |
| UI33-G-4   | Assignment: `slot = expr`           | Updates a state slot.                                                                                            |
| UI33-G-5   | `when COND { ... }`                | Conditional block (no `else` initially — keep grammar minimal).                                                  |
| UI33-G-6   | Arithmetic + comparison ops        | `+ - * / == != < > <= >= && \|\|` over numbers/booleans. No coercion.                                            |
| UI33-G-7   | `or` operator on map lookup        | `cells[k] or "default"` — keeps null-handling explicit. Lowers to backend-idiomatic `?.` / `??`.                |
| UI33-G-8   | `ext NAME ( args ) -> type`        | Calls a value-returning extension point. Returns the host's value (or the `default { ... }` block).             |
| UI33-G-9   | `extension-point NAME (params) -> type { default { ... } }` | Declares a value-returning override hook with a fallback body.                               |
| UI33-G-10  | Built-in fns                       | `min`, `max`, `abs`, `cell-key`, `length`, ... — small standard library. Each lowers per backend.               |
| UI33-G-11  | `let NAME = expr`                  | Local binding for action-scoped intermediate values. Read-only, never re-bound. Scoped to the enclosing `{ }`.   |
| UI33-G-12  | Tuple expression `( a , b )` + projection `t . 0` / `t . 1` | Cheap multi-return without record types. Two-element tuples only in v0.1.0.     |
| UI33-G-13  | `ext NAME ( args )` (no `-> type`) + `extension-point NAME ( ... ) { default { } }` | Void extension point — host notification hook. Default body is empty. |

### 3.3 Per-backend emission

Each backend gets its own `mosaic-emit-core-<backend>` crate
mirroring the existing `mosaic-emit-<backend>` family.

| Backend       | State container             | Reducer shape                                | Extension points become           |
|---------------|-----------------------------|----------------------------------------------|-----------------------------------|
| React         | `useReducer` over `GridState` interface | Discriminated-union action TS type, switch reducer | Host functions passed into `useGridCore({...})` |
| SwiftUI       | `@Observable final class GridCore` | Methods on the observable                    | Closure properties                |
| Flutter       | `ChangeNotifier` subclass     | Methods that call `notifyListeners()`        | `final void Function(...) onCommit;` fields |
| Qt QML        | `QObject` with `Q_PROPERTY` + signals | `Q_INVOKABLE` slots                         | Signal handlers + JS callback props |
| HTML          | Plain ES module + `EventTarget` | Functions that emit a `CustomEvent`          | `addEventListener('commit', fn)`  |
| WebComponent  | Same module, plus a registry for shadow-DOM scoping | Same                                         | Same                              |
| XAML          | `ObservableObject` + `INotifyPropertyChanged` | Methods that raise property-changed         | Virtual methods or delegate props |

The emission strategy is **idiomatic per backend** while preserving
exact action semantics across backends. A reducer expressed once in
`.core` produces seven structurally identical state machines, each
written in its target language's natural pattern.

### 3.4 Override mechanisms — three layers of escape hatch

Business logic changes. Cores ship sensible defaults but every host
will eventually need to bend something. The spec offers three layers
of override, in increasing power and decreasing fine-grain control:

#### Layer 1 — Named extension points (the everyday case)

This is what §3.1's example uses throughout. Core author declares
`extension-point NAME ( ... ) -> type { default { ... } }` at every
meaningful decision; host writes a function with the matching name in
its host file. No DSL change, no grammar, just a function.

| Pros                                                           | Cons                                                                                              |
|----------------------------------------------------------------|---------------------------------------------------------------------------------------------------|
| Typed, named, intentional. Host's IDE auto-completes the hook. | Only the points the core author thought to expose are overridable.                                |
| Default body is statically known — easy reasoning.             | Adding a new extension point requires a core release (which downstreams must opt into).           |
| Composable — extension-point bodies can call other ext points. | If a real-world need lands between two ext points, the host has to either request a new one or fall to Layer 2/3. |

**Rule of thumb for core authors.** Declare an extension point at
every `// TODO: should this be configurable?` moment. Cores should be
generous — extension-point declarations have near-zero runtime cost
(they're function dispatch) and saying "you can override this" up
front is cheaper than retrofitting later.

#### Layer 2 — Dispatcher-level action override (the escape hatch)

When the core's decomposition doesn't match the host's reality, the
host can replace a whole action body from the `.disp` file:

```
dispatcher VisiCalc {
  uses core mosaic-core-grid as grid

  // Replace the editCommit action body entirely. The host's body has
  // access to the same state slots and built-ins as the core's body,
  // PLUS the host can call into the original via `super.editCommit`.
  override action grid.editCommit {
    when edit-row != -1 {
      let stored = parse-and-evaluate ( edit-content )
      let row-tag = compute-row-tag ( edit-row , cells )            // host extra
      cells [ cell-key ( edit-row , edit-col ) ]      = stored
      cells [ "tag:" + cell-key ( edit-row , edit-col ) ] = row-tag // host extra slot
      edit-row     = -1
      edit-col     = -1
      edit-content = ""
    }
  }
}
```

This is the escape hatch. Use it when:

- The core's extension points don't carve out the right shape AND
- Bumping the core to add a new extension point isn't an option (you
  don't own the core, or you need to ship today).

The override body lives in the `.disp` file, not in host code, so it
still benefits from the per-backend emission machinery. It also means
the override is **statically visible** — anyone reading the dispatcher
file sees that the host has diverged from the core's default, and
upgrading the core is a deliberate "do my overrides still apply?"
exercise rather than a silent breakage.

| Pros                                                           | Cons                                                                                              |
|----------------------------------------------------------------|---------------------------------------------------------------------------------------------------|
| No core change needed. Host owns the divergence.               | Override must keep state-shape compatibility with the core (no rogue slots).                      |
| Static, in-tree, reviewable.                                   | Defeats some of "framework reasons about the business logic" — overridden actions are opaque to core-level analysis. |
| `super.NAME` lets the override delegate to the original.       | Upgrading the core may silently change semantics around the overridden action.                    |

#### Layer 3 — State default overrides (configuration)

The cheapest and most common kind of "I want different behaviour":
change a default. Settable on the `uses core ... as ...` line:

```
dispatcher VisiCalc {
  uses core mosaic-core-grid as grid (
    total-rows = 1000 ,
    total-cols = 100 ,
    edit-row   = -1                  // unchanged from core default, but explicit
  )
}
```

This is plain configuration — no DSL machinery, no override body, just
overriding initial values. Useful for sizing, default modes, feature
flags exposed as state slots.

The core declares which state slots are publicly-overridable by
marking them with `config` instead of relying on convention. (Slots
without `config` can still be touched by Layer 2 overrides but won't
appear in IDE autocomplete on `uses core (...)`.) Sketch:

```
state {
  config total-rows  : number = 100
  config total-cols  : number = 26
  cells              : map<text, text>      // not config — internal
  edit-row           : number = -1           // not config — internal
  ...
}
```

#### How the three layers compose

A real-world VisiCalc pilot might use all three:

```
dispatcher VisiCalc {
  uses core mosaic-core-grid as grid (
    total-rows = 10000 ,                                       // Layer 3
    total-cols = 200
  )

  // Layer 1 hooks (declared in host file, not here):
  //   on-commit          → call formula engine
  //   post-commit-cursor → move right on Tab, down on Enter
  //   can-edit           → check workbook permissions

  // Layer 2 escape hatch:
  override action grid.editCommit {
    // Custom: write to both cell store AND undo log atomically
    when edit-row != -1 {
      let stored = ext on-commit ( edit-content , edit-row , edit-col ) -> text
      let undo-frame = capture-undo ( cells , edit-row , edit-col )
      cells [ cell-key ( edit-row , edit-col ) ] = stored
      ext push-undo-frame ( undo-frame )
      super . post-edit-commit-cleanup ( )                     // delegate the rest
    }
  }
}
```

The combination is intentional: configuration goes in §3.5.3, named
hooks go in §3.5.1 (host file), heavyweight divergence goes in §3.5.2
(dispatcher file). Reading the dispatcher tells you exactly how much
the host has diverged from the core.

### 3.6 What `.core` deliberately does NOT include

- **Views.** `.core` cannot lay out widgets. That's `.mll`'s job.
- **Async / effects.** No `await`, no Promise, no I/O. Side effects
  happen only via `ext` calls into host code. This keeps the reducer
  pure and time-travel-debuggable in every backend.
- **Network code.** Same reason.
- **Selectors / derived state.** v0.1.0 keeps it minimal. A future
  spec (UI33-X) adds a `derived NAME = expr` form when we have a real
  need.

## 4. The `.disp` DSL

The Dispatcher is the typed bridge between a `.mil` component's emits
and a `.core`'s actions. It is generated **almost entirely
automatically**: when emit names and action names match, the binding
is implicit. The `.disp` file exists to declare:

- Which cores the project uses
- Which Mosaic components the project mounts
- Explicit binding overrides when the names don't match

### 4.1 Syntactic sketch

```
// code/programs/mosaic/visicalc/visicalc.disp
dispatcher VisiCalc {
  uses core mosaic-core-grid as grid
  uses core mosaic-core-formula-bar as bar

  mounts component Grid          from "./Grid.mil"
  mounts component FormulaBar    from "./FormulaBar.mil"

  // Implicit bindings (auto-derived because names match):
  //
  //   Grid.onNavigate       → grid.navigate
  //   Grid.onFormulaChange  → grid.formulaChange
  //   Grid.onEditCommit     → grid.editCommit
  //   Grid.onEditCancel     → grid.editCancel
  //   FormulaBar.onFormulaChange → grid.formulaChange   (shared slot)
  //   FormulaBar.onCommit   → grid.editCommit
  //   FormulaBar.onCancel   → grid.editCancel

  // Slot bindings — which core slot feeds which component slot:
  bind Grid.viewport-rows     ← grid.viewport-rows
  bind Grid.selected-row      ← grid.selected-row
  bind Grid.selected-col      ← grid.selected-col
  bind Grid.edit-row          ← grid.edit-row
  bind Grid.edit-col          ← grid.edit-col
  bind Grid.edit-content      ← grid.edit-content
  bind FormulaBar.cell-address ← cell-label(grid.selected-row, grid.selected-col)
  bind FormulaBar.formula      ← grid.edit-content
}
```

### 4.2 What this gets us

- One generated file per backend wires every component emit to a core
  action by name. The contract is enforced at codegen time: a `.mil`
  emit with no matching action is a compile error, not a silent
  runtime drop.
- Slot bindings are explicit data flow. The dispatcher emitter knows
  exactly which props each component receives.
- The host writes *no* glue. The `useGridCore()` / `GridCore()`
  instance comes from the generated dispatcher, not from a hand-
  written App entry.

### 4.3 Auto-derivation rules

Implicit binding kicks in when:

- Component emit's camelCased name (without `on` prefix) **equals** a
  core action's name, AND
- The emit's payload type **structurally matches** the action's
  parameter list.

When either fails, the author must add an explicit `bind` line, and
omitting it produces a compile error pointing at the unmatched emit.

### 4.4 Override grammar in `.disp`

Per §3.5, the dispatcher file is also where the host applies state-
default overrides (Layer 3) and action overrides (Layer 2). The
grammar:

| ID         | Construct                                          | Notes                                                                                                                |
|------------|----------------------------------------------------|----------------------------------------------------------------------------------------------------------------------|
| UI33-D-6   | `uses core NAME as ALIAS ( slot = expr , ... )`    | State-default override list. Each slot must be declared `config` in the core. Type-checked against the core.         |
| UI33-D-7   | `override action ALIAS . NAME { body }`            | Replace an action's body. Body uses the same grammar as `.core` action bodies (assignments, `when`, `let`, `ext`).   |
| UI33-D-8   | `super . NAME ( args )` inside an override         | Call into the original core action (or an extension point) from within an override. Solves the "do most of the default + tweak this" case without copy-paste. |

A dispatcher with both Layer 2 + Layer 3 overrides is the heaviest
form a host file should reach for; if it grows past ~4 action
overrides on a single core, that's a strong signal the core itself
should add extension points and the overrides should retire. The
codegen will warn (`UI33-W-1`) when a single dispatcher has more than
3 overrides against a single core.

## 5. Wiring into UI32's `--emit-project`

UI32 already specifies that `mosaic-compile --backend X --emit-project`
produces a runnable shell per backend. UI33 extends this:

```
mosaic-compile --backend react --emit-project \
  --dispatcher code/programs/mosaic/visicalc/visicalc.disp \
  -o code/programs/typescript/visicalc/build/react
```

…produces, in addition to UI32's existing shell artefacts:

| File                                  | Source                                              | What it contains                                                                                       |
|---------------------------------------|-----------------------------------------------------|--------------------------------------------------------------------------------------------------------|
| `src/cores/grid.ts`                   | emitted from `mosaic-core-grid/src/grid.core`       | `interface GridState`, `type GridAction`, `function reducer`, `function useGridCore(overrides)`        |
| `src/cores/formula-bar.ts`            | emitted from `mosaic-core-formula-bar/src/*.core`   | Same shape                                                                                              |
| `src/dispatcher.ts`                   | emitted from `visicalc.disp`                        | `<DispatcherProvider>` wiring all components ↔ cores; props derivation                                  |
| `src/components/Grid.tsx`             | (existing UI29 emission, unchanged)                 | View only                                                                                              |
| `src/components/FormulaBar.tsx`       | (existing UI29 emission, unchanged)                 | View only                                                                                              |
| `src/main.tsx`                        | (UI32 shell, extended)                              | Mounts `<DispatcherProvider>` at root                                                                  |
| `src/host.ts`                         | **new**, host-editable stub                          | Empty extension-point stubs (`export const onCommit = (value, row, col) => value`). Author edits this. |

The host writes *only* `src/host.ts`. Every other file regenerates.

Per-backend equivalents follow the same pattern — cores become
`Sources/App/Cores/Grid.swift`, `lib/cores/grid.dart`, `qml/cores/
Grid.qml`, etc. The shell glue from UI32 is reused without change.

## 6. Reference core: `mosaic-core-grid`

The first core to ship under this spec replaces VisiCalc's hand-
written reducer:

- Source: `code/packages/mosaic-core-grid/src/grid.core` (sketch in §3.1).
- VisiCalc pilot: `code/programs/typescript/visicalc/` adopts `mosaic-core-grid` + the new
  dispatcher. The 168-line `src/app/state.ts` collapses to a 0-line
  generated `src/cores/grid.ts` plus a small `src/host.ts` containing
  only the formula-engine extension point.
- All seven backends migrate in lockstep — Phase 5 of the existing
  cross-backend demo plan (`code/specs/visicalc-cross-backend-demo-
  plan.md`) gets a Phase 5b for the dispatcher migration.

This is the validating end-to-end run. If `mosaic-core-grid` cannot
replace VisiCalc's reducer across all seven backends without
regressions in the functional verification suite (`preview_eval` on
React, screenshot diffing on the others), the spec is wrong.

**Override exercise.** The pilot must also prove all three override
layers (§3.5) on at least one backend. VisiCalc-React is the natural
candidate: it should use a Layer 3 default override (`total-rows =
1000` to demonstrate sizing config), a Layer 1 host hook (`on-commit`
calls a tiny in-tree expression evaluator like `=1+2` → `"3"` so we
can see business logic actually changing), and a Layer 2 dispatcher
override on `editStart` (e.g. seed editing with `=` if the user types
`=` as the first character). If any of the three layers proves
awkward in practice, the override mechanism gets reworked before the
other six backends consume the same patterns.

## 7. Migration strategy — two-track

Per the rollout decision recorded in this spec's design conversation:

- **Track A (existing pattern).** Today's emitter — `mosaic-compile
  --backend X` producing components only, host writes its own
  reducer — keeps working. All current demos, packages, and downstream
  apps continue to compile and run.
- **Track B (dispatcher pattern).** New: `mosaic-compile --backend X
  --emit-project --dispatcher path/to/X.disp` produces the full shell
  with cores wired in.
- **Pilot.** VisiCalc migrates to Track B as the proving ground.
- **Subsequent migrations.** `mosaic-pkg-toolkit`'s demos and the
  cross-backend-demo plan's other VisiCalc-shaped demos migrate
  incrementally. There is no forced cutover date for downstream apps.
- **Deprecation.** Track A becomes deprecated once every in-repo demo
  has migrated AND the dispatcher pattern has shipped on all seven
  backends. The U29-X1-style removal PR is a separate, future spec.

## 8. Implementation plan — phases & PRs

Roughly ~25 small PRs. Each is independently shippable; the dependency
graph branches aggressively after the grammar lands.

### Phase 0 — spec (this PR)

| PR     | Subject                                                              |
|--------|----------------------------------------------------------------------|
| UI33-1 | This spec doc                                                        |

### Phase 1 — `.core` grammar + IR

Sequential within the phase; each PR is small.

| PR        | Subject                                                       |
|-----------|---------------------------------------------------------------|
| UI33-G-1  | `core NAME { ... }` top-level lexer/parser/IR                 |
| UI33-G-2  | `state { ... }` block                                          |
| UI33-G-3  | `action NAME (params) { body }`                               |
| UI33-G-4  | Assignment statements                                          |
| UI33-G-5  | `when COND { ... }` conditional                                |
| UI33-G-6  | Arithmetic + comparison operators                              |
| UI33-G-7  | `or` operator + map lookup                                     |
| UI33-G-8  | `ext` call to extension point                                  |
| UI33-G-9  | `extension-point NAME (...) -> type { default { ... } }`       |
| UI33-G-10 | Built-in fns standard library                                  |

### Phase 2 — `.disp` grammar + IR

| PR        | Subject                                                       |
|-----------|---------------------------------------------------------------|
| UI33-D-1  | `dispatcher NAME { ... }` top-level                             |
| UI33-D-2  | `uses core NAME as ALIAS`                                       |
| UI33-D-3  | `mounts component NAME from PATH`                               |
| UI33-D-4  | Auto-derivation of implicit emit ↔ action bindings              |
| UI33-D-5  | `bind component.slot ← core.slot` slot wiring + expressions     |
| UI33-D-6  | State-default override list on `uses core (...)` (Layer 3)      |
| UI33-D-7  | `override action ALIAS.NAME { body }` (Layer 2)                 |
| UI33-D-8  | `super.NAME(args)` delegation inside override body              |
| UI33-D-9  | `config` slot marker in `.core` + IDE-completion metadata for `.disp` (Layer 3 type-check) |
| UI33-D-W1 | Codegen warning when a dispatcher has > 3 overrides per core (signals core should add extension points) |

### Phase 3 — first end-to-end backend (React)

| PR        | Subject                                                       |
|-----------|---------------------------------------------------------------|
| UI33-E-react-1 | `mosaic-emit-core-react` — state + reducer + hook        |
| UI33-E-react-2 | `mosaic-emit-dispatcher-react` — Context provider         |
| UI33-E-react-3 | `--emit-project` React shell extension to consume cores  |

### Phase 4 — reference core + pilot

| PR        | Subject                                                       |
|-----------|---------------------------------------------------------------|
| UI33-R-1  | `mosaic-core-grid` v0.1.0 (the `.core` file + package metadata) |
| UI33-V-react | VisiCalc-React migrated onto the dispatcher pattern         |

### Phase 5 — remaining backends (parallelizable after Phase 3 lands)

For each backend X ∈ {swiftui, flutter, qt, html, webcomp, xaml}:

| PR                | Subject                                                  |
|-------------------|----------------------------------------------------------|
| UI33-E-X-1        | `mosaic-emit-core-X`                                     |
| UI33-E-X-2        | `mosaic-emit-dispatcher-X`                               |
| UI33-E-X-3        | `--emit-project` shell extension for X                   |
| UI33-V-X          | VisiCalc-X migrated onto the dispatcher pattern          |

### Phase 6 — additional cores (parallelizable, post-pilot)

| PR        | Subject                                                  |
|-----------|----------------------------------------------------------|
| UI33-C-form     | `mosaic-core-form` (controlled inputs + validation)|
| UI33-C-list     | `mosaic-core-list` (selection, sort)               |
| UI33-C-tree     | `mosaic-core-tree`                                 |
| UI33-C-tabs     | `mosaic-core-tabs`                                 |
| UI33-C-router   | `mosaic-core-router`                               |

## 9. Risks & open questions

### 9.1 Risks

- **`.core` grammar scope creep.** The temptation to add async, derived
  state, selectors, computed properties, and a full expression
  language is real. v0.1.0 deliberately stays minimal — the
  validating question is "does it express VisiCalc's reducer **and
  exercise all three override layers**?". If yes, ship; defer
  everything else.
- **Extension-point granularity calibration.** §3.5.1 advises core
  authors to be generous with extension points. Too few and the
  Layer 2 escape hatch becomes the default; too many and the core
  feels like a parameter soup. The pilot will iterate the
  granularity for `mosaic-core-grid`; future cores should follow the
  pattern it lands on (one extension point per "this is the default
  policy" decision is a reasonable starting heuristic).
- **Cross-backend semantic drift.** Seven emitters must produce
  semantically identical state machines from one `.core` source.
  This needs a shared property-test suite where every backend
  consumes the same action stream and reports the same final state.
  Without it the cores will drift the moment a feature lands in one
  emitter first.
- **Extension-point ergonomics.** If hosts find the `ext` mechanism
  awkward (passing closures through generated code in some target
  languages can be unidiomatic), adoption stalls. Each backend's
  Phase 5 PR must include a worked extension-point example, not just
  a generated empty stub.

### 9.2 Open questions deferred to follow-up specs

- **Time-travel debugging.** Reducer purity makes this trivially
  possible. A future UI33-D-* (debugger) spec defines a generic action
  log + replay tool that works across all seven backends.
- **State persistence.** Should the dispatcher know how to serialise
  state to localStorage / UserDefaults / shared_preferences /
  QSettings / etc.? Defer until at least two demos request it.
- **Multi-instance / nested dispatchers.** Today's `.disp` assumes one
  dispatcher per project. Nested cases (a dialog with its own state
  machine inside a page with its own) are real but can ship in v0.2.0.
- **Hot reload semantics.** When the `.core` changes, what happens to
  the in-memory state? Reset? Migrate? Defer.

## 10. Non-goals

- This spec does **not** define a state-management library to be
  installed via npm/pub/SwiftPM/etc. The cores are generated from
  `.core` source per project. Hand-written libraries that resemble
  cores (Redux, Bloc, etc.) remain perfectly usable on Track A.
- This spec does **not** define IPC or multi-process state. A core
  is in-process state machinery only.
- This spec does **not** mandate Track-A removal. The two-track
  rollout in §7 leaves Track A indefinitely supported until a future
  removal spec is written.

## 11. Why this fits the larger arc

This work advances three of the project's standing themes:

1. **Intelligence in the framework, not the weights**
   (`project_total_coverage_forces_reasoning.md`). Lifting reducers
   into a DSL means one specification produces correct
   implementations across seven backends — the framework does the
   reasoning the model would otherwise have to redo per backend.
2. **Every-token-represented rule.** A `.core` file is a single
   reducer specification; every state mutation, every action handler,
   every extension point becomes a tracked artefact in every
   backend's emission. There is no shadow business logic anywhere.
3. **Dev tools for free**
   (`project_dev_tools_for_free.md`). Reducer purity gives time-
   travel debugging, action logging, and reducer replay for free
   across every backend (UI33-D-* follow-up).

---

*End of spec.*
