# UI33-rewrite — Unified architecture for Mosaic UI

> **Status.** Canonical. Supersedes the original UI33 spec
> (`code/specs/UI33-dispatcher-and-cores.md`), UI33-S (event-handling
> survey), UI33-S2 (platform-landscape review with §9 / §10 / §11
> amendments), and UI33-S3 (Reactism critique).
>
> **Reading guide.** §1 is the architecture in one diagram and is
> sufficient context for most readers. §2–§5 specify the IR; §6
> specifies the Mosaic-owned Flux runtime; §7 specifies emission
> per backend; §8 covers DevTools; §9 is the implementation roadmap.
> §10 lists what each previous doc contributed.

---

## 1. The architecture, in one diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│  Host application code (target language)                             │
│    src/main.{ts,swift,kt,dart,cs,cpp,html}                           │
│    src/actions/ — host-edited <mosaic:custom> blocks                 │
├─────────────────────────────────────────────────────────────────────┤
│  Auto-generated per backend (build artefacts; host may edit)        │
│    actions/* — one class per action with apply(state) → state       │
│    state.* — type/struct/interface declaration                       │
│    dispatcher.* — store wiring + subscription                        │
│    components/* — views from .mll                                    │
├─────────────────────────────────────────────────────────────────────┤
│  mosaic-flux-<backend> runtime library (Mosaic-owned)                │
│    MosaicStore<State, Action>                                        │
│    MosaicAction<State> base/protocol/interface                       │
│    Fine-grained subscription primitives                              │
│    DevTools hooks (action log, time-travel, state diff)              │
├─────────────────────────────────────────────────────────────────────┤
│  Mosaic IR (the author writes only these)                            │
│    *.mil — component contract: slots (reads) + actions (fires)       │
│    *.mll — view layout (kernel primitives + composed components)     │
│    *.msl — view style                                                │
└─────────────────────────────────────────────────────────────────────┘

         ↑                              │
         │ state subscription           │ dispatch(action)
         │ (fine-grained)               ▼
         └──────────────────────────────────────────────
                                                        
            Strict unidirectional flow. View layer is
            read-only by IR construction. Every state
            change is a round trip: action object →
            dispatcher → apply() → new state → render.
```

### The five invariants

1. **The view layer is read-only.** A `.mll` declares slot reads and action fires; it has no mutation syntax. Direct mutation from a component is unrepresentable.
2. **Strict Flux.** Every state change passes through `dispatch(action)`. There is no escape hatch — no two-way binding, no "internal component state."
3. **Action object = source of truth.** Each action is a class (or platform equivalent) with payload + `apply(state) → state`. The dispatcher routes by calling `action.apply(state)`. No switch statement, no separate reducer registry.
4. **Component-only exposure.** Authors write components, never platform widgets, never pixel-drawing APIs. Components compose kernel primitives; kernel primitives map to native widgets per backend.
5. **Single source of truth for action shape.** An `action` declared in any `.mil` IS the dispatcher's action type. No parallel declaration in a store file. No mismatch possible.

### The class of bug this prevents

PR #4594 fixed three independent failures in VisiCalc's React demo:
- Cell input had no `onChange` → typing did nothing
- `editCommit` dispatch had no value payload → cell received `undefined`
- Per-cell click had no `onClick` wiring → clicks did nothing

All three shared one cause: the contract between component declarations, emitted output, and host reducer was hand-wired across three files and the framework verified nothing.

Under this architecture, all three become **compile errors**:
- A `.mll` HostInput bound to a writable slot must declare its corresponding action — emitter refuses otherwise.
- The action's payload comes from the action's class constructor signature; the dispatcher can't forget to pass it.
- A `.mll` Box wanting click behaviour must declare `onClick: dispatch: ActionClass(args)` — the action class's required constructor args are checked at codegen.

---

## 2. The IR — three file types

The author writes only:

| File | Purpose |
|---|---|
| `*.mil` | Component contract: which slots the component reads from state, which actions it can fire |
| `*.mll` | View layout: composition tree referencing kernel primitives, components, control flow, slot reads, action dispatches |
| `*.msl` | View style: per-platform style mappings |

There is no `.store`, `.core`, `.disp`, `.observable`, `.model`, `.bind`, or any other Mosaic IR file type. Everything else is either auto-generated per backend or written by the host in target language.

### 2.1 The `.mil` file

```
component Grid {
  // Slots — state fields the component reads (subscription targets).
  // Declaring a slot here means: when this slot's value changes in
  // the store, this component re-renders. Subscription is automatic
  // and fine-grained.
  slot column-headers : list<text>
  slot viewport-rows  : list<list<text>>
  slot selected-row   : number = 0
  slot selected-col   : number = 0
  slot edit-row       : number = -1
  slot edit-col       : number = -1
  slot edit-content   : text   = ""
  
  // Actions — events the component can fire. These ARE the action
  // type; there is no parallel declaration. Defaults from slots
  // become initial state in the store.
  action navigate(row: number, col: number)
  action editStart(row: number, col: number)
  action editCommit
  action editCancel
  action formulaChange(value: text)
}
```

### 2.2 The `.mll` file (excerpt)

```
layout Grid {
  HostTable [sheet] {
    HostTableBody {
      For (each: slot: viewport-rows, as: row, index: r) {
        Row [data-row] {
          For (each: row, as: v, index: c) {
            Box [cell] (
              state-when-selected: (r == selectedRow && c == selectedCol),
              state-when-editing:  (r == editRow && c == editCol),
              onClick: dispatch: navigate(row: r, col: c)   ← dispatch syntax
            ) {
              If (when: (r == editRow && c == editCol)) {
                HostInput (
                  value:    slot: edit-content,
                  onChange: dispatch: formulaChange(value: <input-value>),
                  onCommit: dispatch: editCommit,
                  onCancel: dispatch: editCancel
                )
              }
              Else {
                Text (content: (v))
              }
            }
          }
        }
      }
    }
  }
}
```

The `dispatch: ActionName(args)` syntax replaces the previous `emit: onX` indirection. The action is invoked directly with its required payload at the dispatch site. The emitter verifies arg names and types against the action's declaration in `.mil`.

### 2.3 The `.msl` file

Unchanged from existing UI28-1 / UI29 designs. Per-part style mappings, per-state conditional spreads, per-platform variants.

---

## 3. Strict Flux semantics

### 3.1 The round trip

For every user interaction:

```
1. View fires action (e.g., a click handler calls dispatcher.dispatch(new Navigate(5, 3)))
2. Dispatcher receives the action object
3. Dispatcher calls action.apply(currentState) → newState
4. Dispatcher swaps state and notifies subscribers
5. Subscribers (component selectors) check if their watched slots changed
6. Affected components re-render with new state
```

Every keystroke makes this round trip. There is no shortcut. The view layer cannot mutate state directly. The performance cost is bounded by **fine-grained subscription** (§3.3).

### 3.2 The architectural payoff

The strict-Flux invariant delivers:

| Property | How |
|---|---|
| Undo / redo for free | Store the action stream; replay reducer with truncated stream |
| Time-travel debugging | Action objects are recorded; clicking an old action replays from there |
| Action audit log | Every state change has a typed, named action object behind it |
| Cross-platform DevTools | One protocol works across all 7 backends because runtime is uniform |
| UI development with dummy data | A view bound to `dispatcher.state` works against a hand-rolled mock store; no reducer logic needed |
| Bug prevention | Mismatches between view and reducer become compile errors (§1) |
| Predictable rendering | Given (state, action) the next state is always the same |
| Easy testing | Each action class's `apply()` is a pure function; trivially testable |

### 3.3 Fine-grained subscription is automatic

Performance objection: "every keystroke goes through dispatch — that's slow."

The IR enforces fine-grained subscriptions: each component subscribes only to the slots it reads (declared in its `.mil`). When `edit-content` changes, only components that listed `edit-content` as a slot re-render. For VisiCalc:

| Action | Components that re-render |
|---|---|
| `FormulaChange("h")` (typing in a cell) | The inline `HostInput` (reads edit-content) + the FormulaBar (reads edit-content via selector). 2 components out of 2,600 cells. |
| `Navigate(5, 3)` | All cells whose `state-when-selected` predicate flipped (the old selected cell + the new selected cell). 2 components. |
| `EditCommit()` | The 1 cell whose `state-when-editing` predicate flipped + the 1 cell whose stored value just changed + the FormulaBar. 3 components. |

The selector layer the emitter generates uses each backend's blessed mechanism (React's `useSelector`, SwiftUI's `WithViewStore`, Compose's `collectAsState`, etc.). Per-keystroke is cheap.

### 3.4 The one-active-editor invariant

VisiCalc requires that at most one cell can be in edit mode at any time. The state shape enforces this structurally:

```
slot edit-row : number = -1   //  -1 means "no cell is being edited"
slot edit-col : number = -1
```

A single `(edit-row, edit-col)` pair can encode at most one position. The `.mll`'s `If (when: r == editRow && c == editCol)` predicate is true for at most one cell per render. Two cells simultaneously in edit mode is unrepresentable.

The HostInput emitter auto-focuses on mount; transitioning from cell A1 to cell B5 unmounts A1's input and mounts B5's with focus.

This pattern generalises: data tables with row editing, trees with rename, lists with inline editing. All use the single `(editor-id: T = sentinel)` shape.

---

## 4. Action declarations (the unified `action` keyword)

The `.mil` `action` keyword serves both purposes that older designs split:
- **Component-level**: "this component can fire this action"
- **Store-level**: "this action's signature is part of the action union"

Both meanings reduce to one declaration. The auto-generated Store has one entry per unique `action` name across all `.mil` files in the project. Multiple components can declare the same `action` — they share the same generated class.

### 4.1 Where actions come from

The dispatcher's action union is the disjoint sum of:

1. **Component-declared actions.** Every `action NAME(params)` in any `.mil`.
2. **Host-injected actions.** Actions the host dispatches directly from non-view code — typically a keyboard handler, an effect, or middleware. The host declares these in a project-level config file (`mosaic-project.toml`) or just in the action class file directly.

Most actions come from (1). Examples of (2) for VisiCalc: a window-level `KeyboardNavigate(direction)` action, a startup `LoadData(json)` action.

### 4.2 Action lifecycle

Author writes `action navigate(row: number, col: number)` in `Grid.mil`.

Emitter generates per backend:
- A class `Navigate` with `(row, col)` payload
- Inclusion in the typed action union
- A stubbed `apply(state) → state` method

Author optionally edits the generated class (only for actions whose `apply` body can't be auto-inferred — see §5).

When a `.mll` dispatch site fires `dispatch: navigate(row: r, col: c)`, the emitter generates code that constructs `new Navigate(r, c)` (or platform equivalent) and passes it to the dispatcher.

The dispatcher calls `action.apply(currentState)`, gets the new state, notifies subscribers.

---

## 5. Auto-generation — the Command Pattern

For every action declared in any `.mil`, the emitter produces one class file per backend. Each class encapsulates the action's payload and its state transform.

### 5.1 The pattern across backends

```typescript
// React: src/actions/Navigate.ts
import type { MosaicAction } from "@mosaic/flux-react"
import type { GridState } from "../state"

export class Navigate implements MosaicAction<GridState> {
  constructor(public readonly row: number, public readonly col: number) {}
  
  apply(state: GridState): GridState {
    // <mosaic:custom>
    return {
      ...state,
      editRow: -1, editCol: -1, editContent: "",
      selectedRow: this.row, selectedCol: this.col,
    }
    // </mosaic:custom>
  }
}
```

```swift
// SwiftUI: Sources/Actions/Navigate.swift
import MosaicFlux

struct Navigate: MosaicAction {
  let row: Int
  let col: Int
  
  func apply(to state: GridState) -> GridState {
    // <mosaic:custom>
    var s = state
    s.editRow = -1
    s.editCol = -1
    s.editContent = ""
    s.selectedRow = row
    s.selectedCol = col
    return s
    // </mosaic:custom>
  }
}
```

```kotlin
// Compose: actions/Navigate.kt
import org.mosaic.flux.MosaicAction

data class Navigate(val row: Int, val col: Int) : MosaicAction<GridState> {
  override fun apply(state: GridState): GridState =
    // <mosaic:custom>
    state.copy(
      editRow = -1, editCol = -1, editContent = "",
      selectedRow = row, selectedCol = col,
    )
    // </mosaic:custom>
}
```

```dart
// Flutter: lib/actions/navigate.dart
import 'package:mosaic_flux/mosaic_flux.dart';

class Navigate extends MosaicAction<GridState> {
  final int row, col;
  Navigate(this.row, this.col);
  
  @override
  GridState apply(GridState state) =>
    // <mosaic:custom>
    state.copyWith(
      editRow: -1, editCol: -1, editContent: "",
      selectedRow: row, selectedCol: col,
    );
    // </mosaic:custom>
}
```

```csharp
// XAML / WinUI: Actions/Navigate.cs
using Mosaic.Flux;

public sealed record Navigate(int Row, int Col) : MosaicAction<GridState>
{
    public override GridState Apply(GridState state) =>
        // <mosaic:custom>
        state with
        {
            EditRow = -1, EditCol = -1, EditContent = "",
            SelectedRow = Row, SelectedCol = Col,
        };
        // </mosaic:custom>
}
```

```cpp
// Qt: actions/Navigate.h
#include <MosaicFlux/MosaicAction.h>

class Navigate : public MosaicAction<GridState> {
public:
  Navigate(int row, int col) : m_row(row), m_col(col) {}
  
  GridState apply(const GridState& state) const override {
    // <mosaic:custom>
    GridState s = state;
    s.editRow = -1;
    s.editCol = -1;
    s.editContent = "";
    s.selectedRow = m_row;
    s.selectedCol = m_col;
    return s;
    // </mosaic:custom>
  }
private:
  int m_row, m_col;
};
```

### 5.2 The `<mosaic:custom>` marker

Each generated `apply()` body lives between `<mosaic:custom>` markers. The emitter regenerates everything *outside* the markers when `.mil` declarations change (e.g., adding a new payload field), but never touches anything *inside* the markers. Host edits survive.

If the host renames or removes the action in `.mil`, the entire file is regenerated (or removed). Renames are detected by the emitter and the custom block is moved to the new file when possible; otherwise it's preserved in a `.deleted/` folder with a warning.

### 5.3 Auto-inference for trivial actions

For actions whose names follow conventions, the emitter generates a complete `apply()` body — no stub, no host edit needed:

| Naming convention | Auto-inferred apply body |
|---|---|
| `action setX(value: T)` | `state.X = value` |
| `action toggleX()` | `state.X = !state.X` |
| `action incrementX()` | `state.X = state.X + 1` |
| `action decrementX()` | `state.X = state.X - 1` |
| `action resetX()` | `state.X = initialState.X` |
| `action clearX()` | `state.X = zero value of T` |
| `action appendToX(item: T)` | `state.X = [...state.X, item]` (when X is a list) |
| `action prependToX(item: T)` | `state.X = [item, ...state.X]` |
| `action removeFromX(at: number)` | `state.X = X.filter((_, i) => i !== at)` |
| `action assocX(key: K, value: V)` | `state.X = {...state.X, [key]: value}` (when X is a map) |
| `action dissocX(key: K)` | omit key from `state.X` map |
| (no match) | stub: `// TODO: implement <action>` |

The naming-convention layer is itself a small DSL; we ship a curated set and authors can declare additional conventions in `mosaic-project.toml`.

For VisiCalc, `formulaChange(value: text)` matches `setX` (it sets `editContent` via `formulaChange ↔ edit-content` mapping). The emitter auto-generates:

```typescript
// FormulaChange.ts — fully auto, no host edit needed
export class FormulaChange implements MosaicAction<GridState> {
  constructor(public readonly value: string) {}
  apply(state: GridState): GridState {
    return { ...state, editContent: this.value }
  }
}
```

For `navigate(row, col)`, `editCommit()`, `editStart(row, col)`, and `editCancel()`, no naming convention matches — the emitter writes stubs and the host edits the `<mosaic:custom>` blocks once.

### 5.4 The dispatcher (also auto-generated)

```typescript
// src/dispatcher.ts — auto-generated, no <mosaic:custom> blocks
import { createMosaicStore, MosaicAction } from "@mosaic/flux-react"
import type { GridState } from "./state"

export const initialState: GridState = {
  columnHeaders: Array.from({ length: 26 }, (_, i) => String.fromCharCode(65 + i)),
  viewportRows: [/* host computes */],
  selectedRow: 0, selectedCol: 0,
  editRow: -1, editCol: -1, editContent: "",
}

export const dispatcher = createMosaicStore<GridState, MosaicAction<GridState>>(initialState)
```

The dispatcher is `MosaicStore` from the runtime library (§6). Its `dispatch(action)` method calls `action.apply(state)`, swaps state, notifies subscribers. No switch statement.

### 5.5 What's auto-generated total — VisiCalc-React

```
src/
├── components/                    ← from .mll/.msl
│   ├── Grid.tsx                   (auto, never edited)
│   └── FormulaBar.tsx             (auto, never edited)
├── actions/                       ← one class per action
│   ├── Navigate.ts                (host edits <mosaic:custom>)
│   ├── EditStart.ts               (host edits)
│   ├── EditCommit.ts              (host edits)
│   ├── EditCancel.ts              (auto-inferred, never edited)
│   └── FormulaChange.ts           (auto-inferred, never edited)
├── state.ts                       ← auto-generated GridState type
├── dispatcher.ts                  ← auto-generated MosaicStore wiring
└── main.tsx                       ← host writes 5 lines: mount App with dispatcher
```

Total host-authored target-language code for VisiCalc: ~30 lines in `main.tsx` + 3 `<mosaic:custom>` blocks across the 3 non-trivial action files. Everything else is generated.

---

## 6. The `mosaic-flux-*` runtime libraries

### 6.1 One library per backend

| Library | Lang | Lines (estimated) | Maintenance owner |
|---|---|---|---|
| `mosaic-flux-react` | TypeScript | ~250 | Mosaic |
| `mosaic-flux-swiftui` | Swift | ~300 | Mosaic |
| `mosaic-flux-compose` | Kotlin | ~300 | Mosaic |
| `mosaic-flux-flutter` | Dart | ~280 | Mosaic |
| `mosaic-flux-xaml` | C# | ~350 | Mosaic |
| `mosaic-flux-qt` | C++ / QML | ~400 | Mosaic |
| `mosaic-flux-html` | JS | ~150 | Mosaic |
| `mosaic-flux-webcomponent` | JS | (shares html core) | Mosaic |

### 6.2 The common API surface (illustrated in TypeScript)

```typescript
// MosaicAction protocol
export interface MosaicAction<State> {
  apply(state: State): State
}

// MosaicStore
export class MosaicStore<State, Action extends MosaicAction<State>> {
  constructor(initial: State, middleware?: Middleware<State>[])
  
  dispatch(action: Action): void
  get state(): State
  
  // Fine-grained subscription
  subscribe<T>(selector: (s: State) => T, fn: (value: T) => void): () => void
  
  // DevTools integration
  enableDevTools(): void
}

// Middleware (for cross-cutting like logging, persistence, async)
export type Middleware<State> =
  (action: MosaicAction<State>, prev: State, next: State) => void

// Selectors for derived state
export const createSelector = <State, T>(
  fn: (s: State) => T,
  equalityFn?: (a: T, b: T) => boolean
) => /* memoized selector */
```

Each backend library exposes the same five concepts (`MosaicAction`, `MosaicStore`, `dispatch`, `subscribe`, `middleware`) in the platform's natural idiom. The shape is uniform; the syntax is native.

### 6.3 Per-backend integration with the platform's reactive system

Each runtime integrates with its platform's native reactive system so that components subscribed to the store re-render through the platform's blessed mechanism:

| Backend | Integration |
|---|---|
| React | `useMosaicSelector(s => s.editContent)` hook (internally uses `useSyncExternalStore`) |
| SwiftUI | `@MosaicObserved var store: MosaicStore` property wrapper (uses Swift's Observation framework) |
| Compose | `store.observeAsState(s => s.editContent)` (uses `StateFlow`) |
| Flutter | `MosaicSelector<GridState, String>(selector: (s) => s.editContent, builder: ...)` (uses `StreamBuilder`) |
| XAML | `INotifyPropertyChanged` integration; bind `{x:Bind Dispatcher.State.EditContent}` |
| Qt | `Q_PROPERTY(QString editContent READ getEditContent NOTIFY editContentChanged)` |
| HTML / Web Component | `dispatcher.subscribe(s => s.editContent, (val) => ...)` or `<mosaic-bind>` element |

### 6.4 Middleware

Middleware sees every `(action, prevState, nextState)` triple. Common use cases:

- **Logger**: log every dispatched action with timestamp and state diff
- **Persistence**: serialize chosen slots to disk / localStorage / etc.
- **Async effects**: when action X completes, perform side effect Y (via extension points)
- **Analytics**: report user-significant actions to telemetry
- **Validation**: assert invariants on the new state; throw in dev mode

Middleware is host-authored target-language code. Each runtime exposes a `MosaicStore.use(middleware)` registration.

### 6.5 DevTools hooks

When `dispatcher.enableDevTools()` is called (typically in dev builds), the runtime starts emitting a structured event stream on a known channel:

| Backend | Channel |
|---|---|
| React / HTML / Web Component | `window.postMessage` to a `mosaic-devtools` browser extension OR a local WebSocket on `ws://localhost:9229` |
| SwiftUI | Local TCP socket on port `9229` |
| Compose / Flutter | Local TCP socket on port `9229` |
| XAML | Named pipe `\\.\pipe\mosaic-devtools` |
| Qt | Local TCP socket on port `9229` |

The structured event format is the same across all backends (§8). One Mosaic DevTools desktop application can attach to any of them.

---

## 7. Platform tiering — what we emit and what we don't

### 7.1 Tier 1A — native-widget smart emit (5 platforms)

| Platform | Reach | Component widgets bind to |
|---|---|---|
| **React DOM** | Web (browsers) | HTMLElement |
| **SwiftUI** | iOS, iPadOS, macOS, watchOS, tvOS, visionOS | SwiftUI native View |
| **Jetpack Compose** | Android | Material native composable |
| **WinUI XAML** | Windows desktop | WinUI native control |
| **HTML** (server-rendered) | Web (static / SSR) | HTML elements |

These produce **genuinely native end-user experiences**. The user clicking through a Mosaic-built iOS app sees real UIKit/SwiftUI; on Android they see real Material/Compose; on Windows they see real Fluent/WinUI.

The smart emitter is "smart" because it encodes platform idioms deeply: native state containers, native navigation patterns, native accessibility primitives, native theming integration, native project structure.

### 7.2 Tier 1B — cross-platform-consistency smart emit (3 platforms)

| Platform | Reach | Use case |
|---|---|---|
| **Flutter** (primary) | iOS, Android, web, desktop (Win/Mac/Linux) | One binary, identical UI everywhere |
| **Compose Multiplatform** | iOS, Android, desktop, web (Wasm) | Kotlin shops; Android-first teams |
| **Qt** | Desktop, embedded, mobile | C++ teams; industrial / kiosk apps |

These produce **visually consistent output across platforms**. iOS app looks the same as Android app looks the same as web app. Brand-uniform consumer apps, kiosks, enterprise tools choose this tier.

The author picks Tier 1A or 1B per deployment slot. Mosaic component source is identical for both tiers — only the emitter differs.

### 7.3 Tier 2 — view-only (6 platforms, no smart emit)

| Platform | Status |
|---|---|
| React Native | View-only emitter; host wires state |
| Web Components | View-only |
| MAUI | View-only |
| Avalonia | View-only |
| GTK4 | Deferred (Linux desktop, small new-app market) |
| Existing Qt emitter (if not promoted to 1B) | Stays view-only |

Tier 2 emitters generate components without a `mosaic-flux-<backend>` runtime. Hosts wire their own state. Useful for embedding Mosaic UI inside an existing host application that already has its own state architecture.

### 7.4 Out of scope

| Platform | Why not |
|---|---|
| Vue, Svelte, Solid, Angular | Each adds 3–6 months of work; not enough demand |
| UIKit | Covered by SwiftUI for new code |
| Tauri | Covered by HTML + Rust host (host chooses to use Tauri's `invoke`) |
| Slint | Too small a community |
| Game engines (Unity, Unreal) | Different paradigm |

### 7.5 Picking a tier in `mosaic-project.toml`

```toml
[targets.production]
ios     = "swiftui"          # Tier 1A
android = "jetpack-compose"  # Tier 1A
web     = "react-dom"        # Tier 1A
windows = "winui-xaml"       # Tier 1A

[targets.kiosk]
all = "flutter"              # Tier 1B
```

The same `.mil`/`.mll`/`.msl` source compiles to either tier. Emit choice is a deploy-slot decision, not an author decision.

---

## 8. DevTools

### 8.1 The cross-backend protocol

Every `mosaic-flux-<backend>` runtime emits the same structured event format on its DevTools channel:

```json
{
  "kind": "action",
  "ts": 1716923400123,
  "actionType": "Navigate",
  "actionPayload": { "row": 5, "col": 3 },
  "prevState": { /* full state snapshot or diff */ },
  "nextState": { /* same */ },
  "duration_us": 47
}
```

```json
{
  "kind": "subscription",
  "ts": 1716923400125,
  "componentId": "Grid::cell[r=5,c=3]",
  "slots": ["editRow", "editCol", "selectedRow", "selectedCol"],
  "rendered": true
}
```

```json
{
  "kind": "performance",
  "ts": 1716923400130,
  "renderBatch": [/* component IDs */],
  "totalDuration_ms": 12
}
```

### 8.2 The Mosaic DevTools desktop app

One application (Electron or Tauri-based; TBD) attaches to any backend's runtime via its native channel. Provides:

- **Action log** — chronological list of dispatched actions with timestamps, payloads, durations
- **Time-travel** — click any past action to replay state from that point; the runtime reapplies the action stream
- **State inspector** — full state tree with collapsible nodes; diff view between any two states
- **Subscription map** — which components subscribe to which slots; performance flag when many components re-render per action
- **Action replay** — record an action stream on backend A; replay against backend B (useful for cross-platform reproduction)

Because the runtime is uniform, **a single bug reproduction recorded on Web can be replayed against the iOS app**. This is unprecedented for cross-platform UI dev.

---

## 9. Implementation roadmap

### 9.1 Phase 1 — IR grammar (sequential)

| PR | Subject |
|---|---|
| UI33r-G-1 | `.mil` grammar: add `action NAME(params)` declarations |
| UI33r-G-2 | `.mll` grammar: `dispatch: ActionName(args)` syntax |
| UI33r-G-3 | Slot-binding default values (replaces the `state {default}` block from old UI33) |
| UI33r-G-4 | Action-payload type system (text/number/bool/list/map) |
| UI33r-G-5 | Compile-time check: every action invocation matches its declaration |
| UI33r-G-6 | Compile-time check: every slot read in `.mll` matches a declared `slot` |
| UI33r-G-7 | Naming-convention table for auto-inferred actions |

### 9.2 Phase 2 — Mosaic-Flux runtimes (parallelizable)

| PR | Subject |
|---|---|
| UI33r-R-react | `@mosaic/flux-react` v0.1.0 |
| UI33r-R-swiftui | `MosaicFlux` Swift package v0.1.0 |
| UI33r-R-compose | `org.mosaic.flux` Kotlin library v0.1.0 |
| UI33r-R-flutter | `mosaic_flux` Dart package v0.1.0 |
| UI33r-R-xaml | `Mosaic.Flux` NuGet package v0.1.0 |
| UI33r-R-qt | `MosaicFlux` Qt library v0.1.0 |
| UI33r-R-html | `@mosaic/flux-html` v0.1.0 |
| UI33r-R-webcomponent | `@mosaic/flux-webcomponent` v0.1.0 (shares html core) |

Each runtime ships its API as documented in §6.2.

### 9.3 Phase 3 — Smart emitters per Tier 1A backend

Five PR streams in parallel after Phase 1 + the corresponding runtime in Phase 2:

| PR streams | Output per backend |
|---|---|
| UI33r-E-react-1..4 | Action class generator, dispatcher generator, state generator, view emitter integration |
| UI33r-E-swiftui-1..4 | Same shape |
| UI33r-E-compose-1..4 | Same shape |
| UI33r-E-xaml-1..4 | Same shape |
| UI33r-E-html-1..4 | Same shape |

### 9.4 Phase 4 — Tier 1B emitters (Flutter primary, then CMP and Qt)

| PR streams | Output |
|---|---|
| UI33r-E-flutter-1..5 | Flutter smart emit using `mosaic-flux-flutter` |
| UI33r-E-compose-mp-1..3 | Compose Multiplatform variant of UI33r-E-compose |
| UI33r-E-qt-smart-1..3 | Promote existing Qt emitter to smart |

### 9.5 Phase 5 — Pilots

| PR | Subject |
|---|---|
| UI33r-V-react | VisiCalc-React on new architecture (validates Tier 1A web) |
| UI33r-V-swiftui | VisiCalc-SwiftUI (validates Tier 1A Apple) |
| UI33r-V-compose-android | VisiCalc-Compose-Android (validates Tier 1A Android) |
| UI33r-V-flutter | VisiCalc-Flutter (validates Tier 1B consistency philosophy) |

All four pilots must work end-to-end before the architecture is considered validated.

### 9.6 Phase 6 — DevTools

| PR | Subject |
|---|---|
| UI33r-D-1 | DevTools protocol spec (the JSON format from §8.1) |
| UI33r-D-2 | DevTools desktop application v0.1.0 |
| UI33r-D-3 | Cross-backend action replay capability |

### 9.7 Phase 7 — Component catalogue

After UI33r is structurally validated, the component-catalogue work begins (per UI33-S2 §9.6):

| Package | Components |
|---|---|
| `mosaic-pkg-toolkit` | Button, Input, Checkbox, Radio, Select, Switch, Slider, Tabs, Accordion, Dialog, Toast, Tooltip, Popover, Menu, Card, Avatar, Badge, Progress, Spinner |
| `mosaic-pkg-grid` (v0.3.0) | Refactor to UI33r |
| `mosaic-pkg-form` | Form, FormField, FieldError, FieldGroup, Validation |
| `mosaic-pkg-list` | List, ListItem, SortableList, FilterableList |
| `mosaic-pkg-tree` | Tree, TreeNode, ExpandableTreeNode |
| `mosaic-pkg-router` | Route, Link, Outlet |
| `mosaic-pkg-data-table` | DataTable, ColumnDef, SortIndicator |

---

## 10. What each predecessor spec contributed

| Spec | Contribution |
|---|---|
| UI33 (original) | Three-layer architecture; introduced `.core` + `.disp` DSLs (which this rewrite consolidates and renames away) |
| UI33-S | Per-backend event-handling survey; established that "uniform dispatcher contract" framing was wrong on 4 of 7 backends |
| UI33-S2 §0–§8 | Platform-landscape review; introduced Tier 1A / 1B / 2 model; Dropbox cautionary tale |
| UI33-S2 §9 | Component-only exposure constraint; landscape-graveyard analysis collapses |
| UI33-S2 §10 | Native UI per platform default; rejected adopted-stack shortcut for mobile |
| UI33-S2 §11 | Cross-platform consistency as opt-in (Tier 1B); two-philosophy ecosystem |
| UI33-S3 | Reactism critique (subsequently refined: user prefers Flux; this rewrite adopts it explicitly with Mosaic-owned runtime) |

This rewrite is the synthesis. The predecessor specs are kept as historical record; downstream readers should reference this document.

---

## 11. Open questions (deferred)

| ID | Question | When to resolve |
|---|---|---|
| Q3 | Web framework breadth — stay React-only on Tier 1A, or add Vue/Svelte/Solid? | Decide when a real downstream demand emerges; default React-only |
| Q6 | Tier-2 demotion path — comfortable demoting Web Components / MAUI / Avalonia / RN to view-only? | Confirm before any Tier-2 PR |
| Q9 | Component catalogue scope — v0.1.0 catalogue is just Grid + FormulaBar from VisiCalc, or front-load Button/Input/Form/List/Dialog? | Decide before Phase 7 starts |
| Q10 | DevTools UI shell — Electron, Tauri, or web-only? | Before UI33r-D-2 |
| Q11 | Naming-convention table extensibility — projects declare custom conventions in `mosaic-project.toml`, or hard-coded? | Before UI33r-G-7 |
| Q12 | Mosaic-project.toml schema — full schema for tier selection, runtime opts, DevTools config | Before any project compiles end-to-end |

---

## 12. Non-goals

- Vue / Svelte / Solid / Angular emitters (out of scope, §7.4).
- UIKit / AppKit emitters (SwiftUI covers them).
- Pixel-drawing primitive APIs at the IR layer (component-only exposure).
- Built-in async / effects in the action layer (host-side via middleware).
- Component-internal state (every component is a pure projection of store state).
- Reducers expressed in the IR DSL (the `apply` method lives in host target-language code).

---

*End of UI33-rewrite.*
