# UI33-S3 — The `.core` and `.disp` DSLs are Reactisms; the universal IR is `state + operations`

> **Status.** Critique-and-alternatives doc. Triggered by user
> feedback: *"I am still uncomfortable with .core and .disp. Those
> look like Reactisms."*
>
> **Scope.** Argues the `.core` (state + reducer + actions) and
> `.disp` (component-emit → action) DSLs from UI33 / UI33-amendment
> are specifically Redux/Flux/Bloc/TCA DNA, not a universal
> abstraction. Surveys what every UI platform actually uses for
> state-with-behavior; finds the universal pattern is `state +
> operations` (methods on a state container). Proposes three
> alternative IR shapes for the user to pick.
>
> **Recommendation surfaced at end (§7).** Pick one of the three
> options before any UI33-G-* grammar PR starts. The wrong choice
> here makes every emitter awkward on most platforms.
>
> ---

## 1. The critique, stated precisely

UI33 commits to two new DSLs:

- `.core` — declares `state { ... }`, `action NAME(params) { body }`, `extension-point NAME ...`. The mental model: actions are dispatched, the reducer processes them, the store updates, the view re-renders.
- `.disp` — wires a component's emits to a core's actions: "when Grid fires `onNavigate`, dispatch `grid.navigate(r, c)` through the dispatcher."

**This is the Redux model**, with the actions-and-reducer pattern that Redux popularized in the React ecosystem and that flutter_bloc inherited for Dart. It is *one specific architectural family* — not the only one, not the most common one, and not the platform-blessed one on most targets.

Survey of how every other state-management ecosystem actually expresses the same thing:

```
                    "state container"           "operations"        "dispatcher needed?"
─────────────────────────────────────────────────────────────────────────────────────────
SwiftUI             @Observable class           func methods        NO
Jetpack Compose     ViewModel + StateFlow       fun methods         NO
XAML / MVVM Toolkit ObservableObject + props    [RelayCommand] fns  NO
Flutter ChangeNotif class extends ChangeNotif   void methods        NO
Qt QML              QObject + Q_PROPERTY        Q_INVOKABLE methods NO
React (Zustand)     create(set => {…})          methods in store    NO
React (TanStack)    queryClient + caches        mutation methods    NO
React (useReducer)  reducer fn                  actions + reducer   YES (Redux DNA)
React (Redux Tlkt)  createSlice                 reducers + actions  YES (Redux DNA)
Flutter (flutter_bloc) Bloc<E,S>                events + emitters   YES (BLoC DNA)
SwiftUI (TCA)       Reducer struct              actions + reducer   YES (TCA DNA)
```

The ratio is striking. **Most platforms — even React's most modern recommendation — express the same logic as methods on a state container.** No dispatcher, no first-class action type, no reducer function. Just `state` and `operations on state`.

Redux/Bloc/TCA *exist as opt-in libraries* on each platform for hosts who specifically want the unidirectional-flow architecture (good for time-travel debugging, action logging, strict separation). But these are architectural opinions layered on top of the platform's basic state container, not the platform's primitive.

UI33 made the reverse choice: it elevated Redux/Bloc/TCA to the IR layer, forcing every emitter to fight against its platform's blessed pattern to produce dispatcher-shaped code.

## 2. The universal abstraction

The genuinely-cross-platform pattern is:

```
state-container UNIT {
    state field1 : type = default
    state field2 : type = default
    ...
    
    operation method1(arg : type) {
        // mutates fields
    }
    
    operation method2() -> type {
        // reads fields, returns derived value
    }
}
```

This maps trivially to *every* state container in §1's table:

```
SwiftUI:
  @Observable final class UNIT {
      var field1: T1 = default1
      var field2: T2 = default2
      func method1(arg: T) { /* mutate */ }
      func method2() -> T { /* read */ }
  }

Jetpack Compose:
  class UNITViewModel : ViewModel() {
      private val _field1 = MutableStateFlow(default1)
      val field1: StateFlow<T1> = _field1
      fun method1(arg: T) { _field1.value = ... }
  }

XAML (MVVM Toolkit):
  public partial class Unit : ObservableObject {
      [ObservableProperty] private T1 field1 = default1;
      [RelayCommand] private void Method1(T arg) { /* mutate via Field1 */ }
  }

Flutter (ChangeNotifier):
  class Unit extends ChangeNotifier {
      T1 _field1 = default1;
      T1 get field1 => _field1;
      void method1(T arg) { _field1 = ...; notifyListeners(); }
  }

React (Zustand):
  const useUnit = create<UnitState>((set, get) => ({
      field1: default1,
      method1: (arg) => set(s => ({ field1: ... })),
  }));

Qt QML:
  class Unit : public QObject {
      Q_OBJECT
      Q_PROPERTY(T1 field1 READ field1 NOTIFY field1Changed)
  public slots:
      Q_INVOKABLE void method1(T arg) { /* mutate, emit */ }
  };
```

**Each emitter trivially knows how to write the platform-blessed shape.** No mental gymnastics. No "dispatcher contract" to fight. State + operations → idiomatic container on every target.

For the opinionated hosts who *want* Redux/Bloc/TCA, the IR can still emit that shape — it's strictly more expressive than declaring state + methods. A reducer is just an operation that takes an action as its argument; a dispatcher is just a switch over operation names. Hosts opt in (and trade a more verbose authoring surface for the architectural properties they want).

## 3. What this means for `.disp`

`.disp` declares "when this component's emit fires, dispatch this action." In the state-and-operations model, this collapses to:

**When this component's emit fires, call this operation.**

That's a compile-time binding between an emit name and an operation name. It doesn't need a runtime dispatcher object. It's a generator concern — Mosaic's emitter sees:

```
component Grid {
    emit onNavigate(row, col)
}

state-container GridState {
    operation navigate(row, col) { ... }
}
```

…and emits per-backend wiring (in React: `onNavigate={(r, c) => gridState.navigate(r, c)}`; in SwiftUI: `.onNavigate { r, c in gridState.navigate(row: r, col: c) }`; etc.).

Auto-binding kicks in when emit and operation names match (stripped of the `on` prefix). Explicit binding for the rest. **No dispatcher object ever exists at runtime** on any tier-1 backend.

This is a substantial reduction from UI33's framing.

## 4. Three options for the reshape

### Option A — Pure rename, keep the architecture

- Rename `.core` → `.store` (less Reactish) or `.model` (MVVM-friendly) or `.unit`.
- Rename `.disp` → `.bind` or `.wire`.
- Keep the action+reducer architecture inside.
- Each emitter still has to bridge the reducer/dispatcher abstraction to its platform's container.

| Pros | Cons |
|---|---|
| Smallest delta from current UI33 | Doesn't fix the underlying mismatch with platform idioms |
| The "Redux opt-in" path is still default | UI33-S survey's findings still apply: 4 of 7 backends mismatched |
| Easiest migration if any code already uses `.core/.disp` | Cosmetic fix only |

### Option B — Reshape the DSL to `state + operations`, keep the separate file

Replace `.core` and `.disp` with one new DSL:

```
// counter.observable    (or .store, or .model — pick one)
observable Counter {
    state count : number = 0
    
    operation increment() {
        count = count + 1
    }
    
    operation reset() {
        count = 0
    }
    
    operation set(value : number) {
        count = value
    }
}
```

Wiring lives in the existing `.mil` file:

```
// CounterView.mil
component CounterView {
    slot count : number       // displayed
    
    emit onIncrement
    emit onReset
}

// counter.bind          (or wire it implicitly when names match)
bind CounterView.onIncrement → Counter.increment
bind CounterView.onReset → Counter.reset
```

Per-emitter: each observable lowers to the platform's blessed state container (SwiftUI `@Observable`, Compose `ViewModel`, etc.) with methods. Components consume the observable per platform's blessed reactive subscription.

The reducer / action / dispatcher abstractions don't exist in the IR at all. Hosts who *want* them write them in their target language as a host-level concern.

| Pros | Cons |
|---|---|
| Universal abstraction. Every platform's blessed container fits naturally. | Slightly bigger delta from UI33 |
| Eliminates `.disp` as a runtime concept | Hosts who actually want Redux/Bloc/TCA get less native support |
| Each emitter is simpler — no dispatcher generation | Need to spec the override/extension mechanism in a different shape |
| Matches the "smart emitter per backend" thesis from UI33-S2 | Loses some structure that made action logging / time-travel easy |

### Option C — Fold state and operations into `.mil` directly

Don't introduce a new file at all. Extend `.mil` to declare state and operations alongside slots and emits:

```
// Counter.mil
component Counter {
    // Public interface (input/output for parents)
    slot label : text = "Count"
    emit onValueChanged(value : number)
    
    // Internal state (lives in the component instance)
    state count : number = 0
    
    // Operations (called by parents or the view layer)
    operation increment() {
        count = count + 1
        emit onValueChanged(count)
    }
    
    operation reset() {
        count = 0
        emit onValueChanged(count)
    }
}
```

The component is a self-contained unit of `slots + emits + state + operations`. View (`.mll`) and style (`.msl`) describe how the component renders.

The Counter component is also usable as a standalone state container (no view), or as a UI component (with `Counter.mll`), or both.

| Pros | Cons |
|---|---|
| One file format per unit; no new file types | `.mil` grows from "interface declaration" to "stateful unit declaration" — bigger conceptual scope |
| Universal abstraction — same as Option B | Authors who want separation-of-concerns lose it |
| Cleanest mental model: a component owns its state and operations | Pure state containers (no view) feel weird being called "components" |
| Eliminates `.core` AND `.disp` from the file taxonomy | Forces re-thinking what `component` means in the .mil grammar |

## 5. What about Redux / Bloc / TCA fans?

These architectures are not dead. Some apps genuinely benefit from:

- Time-travel debugging
- Action logging and replay
- Strict unidirectional flow
- Architectural pattern consistency across team members trained on it

The state-and-operations IR doesn't preclude these. Hosts who want them implement them at the host layer using the platform's library of choice (Redux Toolkit, flutter_bloc, TCA, etc.). The IR-level operation becomes a thin wrapper that dispatches an action and waits for the reducer to settle.

The reverse direction — bake Redux into the IR — forces every host to use it. Many platforms' developers will hate this and walk away.

The right call is **operations as the IR primitive; Redux as a host-side opt-in**. Same as we treat async (host-side, not IR-side) per UI33 §3.6.

## 6. What this changes about UI33 (and how big the change is)

| Option | UI33 amendments needed |
|--------|------------------------|
| A — rename only | Trivial: find/replace `.core` → new name, `.disp` → new name. Architecture unchanged. ~1 amendment commit. |
| B — reshape to state+ops, separate file | Substantial: rewrite §3 (`.core` DSL) and §4 (`.disp` DSL) entirely. Update §5 (`--emit-project` artefacts to remove dispatcher/reducer skeletons). Update §6 (reference core becomes reference observable). Update §8 (grammar PRs UI33-G-* renumbered and reshaped — fewer of them). ~1 large amendment or full rewrite of UI33. |
| C — fold into `.mil` | Largest: rewrite UI33 entirely around the `state + operations` extension to `.mil`. The dispatcher/core/disp file taxonomy goes away. New PR plan in UI33 §8. ~full rewrite of UI33. |

The good news: **the UI33-S2 platform landscape findings + Tier 1A/1B/2 model + override mechanism principles all stay valid** regardless of which option you pick. Only the IR DSL shape changes.

## 7. Open question — which option?

This needs your call before any UI33-G-* grammar PR starts. Concretely:

- **Option A (rename only)** if you want minimal change and you genuinely want Redux/Bloc/TCA as the default architecture. Honest position: "we picked one architectural opinion and stuck with it."
- **Option B (reshape to state+ops, separate file)** if you want the universal abstraction but want to keep state-container files distinct from view-component files. Probably the most defensible middle ground.
- **Option C (fold into `.mil`)** if you want maximum consolidation — one file format per stateful unit, whether it's a UI component or a pure store. Cleanest if it works; biggest concept-stretch on what `.mil` means.

My honest read: **Option B**. Reasons:

1. The universal-abstraction argument is strongest with B.
2. A state container with no view is a legitimate thing that doesn't fit into `.mil`'s "UI component" framing.
3. Authors who write a `Counter.observable` don't need to learn what a "stateful component without a view" means — they just write an observable.
4. The separate file means the state container's lifecycle is decoupled from the view component's lifecycle (a Counter observable can outlive a CounterView component instance).
5. Option C's file conflation might force awkward "this is a component that has no .mll" situations.

But you have a strong opinion on architectural shape and I want your read before committing.

### A few specific sub-questions if you pick B or C

If Option B or C:

- **What's the file extension / declarator name?**
  - `.observable` (SwiftUI-flavored)
  - `.store` (Zustand / Redux-flavored — still some baggage)
  - `.model` (MVVM-flavored)
  - `.unit` (neutral but vague)
  - `.state` (incomplete — doesn't capture operations)
  - `.entity` (DDD-flavored)
  - Open to suggestions.
- **What's the keyword for the operations?**
  - `operation` (procedural-flavored)
  - `command` (CQRS / MVVM-flavored)
  - `action` (still Reduxish, even if it's a method now)
  - `method` (Swift-flavored)
  - `fn` (Rust-flavored)
- **How explicit are read-only vs. mutating operations?**
  - Mark mutating operations with `mut operation foo() { ... }` (Rust-style)
  - All operations can mutate; reads are just `derived value : type = expr` (Vue-flavored computed properties)
  - Mosaic-specific: no read operations — derived state is always declared via `derived` (avoids the "is this a getter or a method?" ambiguity)

---

*End of UI33-S3. Recommend reading §1 + §4 + §7 even if skipping
everything else.*
