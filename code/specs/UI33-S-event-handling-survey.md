# UI33-S — Native event handling across Mosaic backends (survey)

> **Status.** Survey / research companion to
> `code/specs/UI33-dispatcher-and-cores.md`. Not a spec — but its
> findings should drive the next round of UI33 amendments before
> any UI33-G-* / UI33-E-* grammar or emitter work starts.
>
> **Scope.** For each of the seven Mosaic backends (React, SwiftUI,
> Flutter, Qt QML, HTML, Web Component, XAML), document:
>
>   1. Today's emission — what `mosaic-emit-<X>` actually generates.
>   2. The backend's **native, idiomatic** event-handling story
>      (what the platform's blessed pattern looks like when humans
>      write it).
>   3. The "blessed" reactive-state library the ecosystem has
>      converged on (if any) — TCA, Bloc, MVVM Toolkit, …
>   4. What that means for what UI33's dispatcher + core emitter
>      should produce.
>
> **The thesis we're testing.** UI33 currently assumes all seven
> backends should converge on one architectural shape (one `.core`
> compiled per backend into "the equivalent" reducer + state +
> dispatcher). This survey asks: *is that the right framing*, or
> should each backend get an idiomatic-per-platform shape with the
> `.core` DSL being the IR that lowers to each?

---

## 1. Per-backend deep dive

Seven sections, one per backend. Each follows the same template:
**(a) today's emission**, **(b) native idiom**, **(c) ecosystem
"blessed" library**, **(d) UI33 implication**.

### 1.1 React

**(a) What `mosaic-emit-react` emits today.** JSX callback props,
nothing more. Inputs lower to `onChange={e => dispatch({type: "X",
value: e.target.value})}`; buttons to `onClick={() =>
dispatch({type: "X"})}`. The host is assumed to pass a `dispatch`
prop and to own the reducer. Action shape is a **TS discriminated
union** of `{type: "X"; ...payload}` objects.

**(b) Native React idiom.** Two converged patterns:
1. `useReducer(reducer, initial)` returning `[state, dispatch]`.
   Idiomatic for component-local state machines (small to medium).
   Action shape is exactly the discriminated union we already emit.
2. Context + custom hooks — for app-wide state (`useGridCore()`).
   The hook returns either `{state, dispatch}` or — more
   ergonomically — `{state, actions: { navigate, editCommit, ... }}`
   pre-bound to dispatch.

**(c) Blessed libraries.** Two camps. **Redux Toolkit** (createSlice,
RTK Query) for app-wide stores; **Zustand** for simpler stores. Both
ship discriminated-union actions. **TanStack Query** for server
state. The community has decisively moved away from manual
useReducer for anything app-wide.

**(d) UI33 implication.** Today's emission is already React-
idiomatic at the boundary (`onChange`/`onClick` + tagged-union
dispatch). What's missing is the *container*: a generated
`useGridCore()` hook that wires the reducer + state, plus actions
bound to dispatch so host code calls `actions.navigate(0, 1)`
instead of `dispatch({type: "navigate", row: 0, col: 1})`. Optional
sugar: a `<GridCoreProvider>` Context wrapper for app-wide cores.

---

### 1.2 SwiftUI

**(a) What `mosaic-emit-swiftui` emits today.** Pure value-semantic
View structs. `let dispatch: (NameEvent) -> Void` stored property on
the View, with `.onTapGesture { dispatch(.tap) }` and `.onChange(of:
x) { dispatch(.onChange(value: x)) }` modifiers. **Action shape is
a Swift enum with associated values** — `case navigate(row: Int,
col: Int)`. No `@State`, `@Observable`, or other reactive machinery
generated.

**(b) Native SwiftUI idiom.** SwiftUI is *strongly* reactive — Views
re-render when the state they read changes. The blessed pattern
since iOS 17 / macOS 14:

```swift
@Observable
final class GridCore {
    var state: GridState
    func navigate(row: Int, col: Int) { /* mutate state */ }
    func editCommit() { /* mutate state */ }
}

struct GridView: View {
    @Bindable var core: GridCore
    var body: some View { /* reads core.state.* */ }
}
```

`@Observable` (Swift's Observation framework) replaces `ObservableObject`
+ `@Published` pre-iOS-17. The class IS the dispatcher; method
calls ARE the actions. There is no "dispatch" function in idiomatic
SwiftUI — it's `core.navigate(row, col)` directly.

**(c) Blessed library.** **The Composable Architecture (TCA)** from
Point-Free. Sealed-action enum, reducer function, Store, view store
projection. Used widely in Swift codebases that want explicit
unidirectional data flow. Outside TCA, `@Observable` is the platform's
own answer and is what new code reaches for.

**(d) UI33 implication.** The current emission is unusual for
SwiftUI — passing a `dispatch` closure prop feels like a React
pattern bolted onto Swift. The idiomatic UI33 output would emit:

- A `@Observable final class GridCore` (the core).
- A `GridView: View` that takes `@Bindable var core: GridCore` and
  calls `core.navigate(...)` from `.onTapGesture` etc.
- Optional: TCA-mode emission for hosts that want sealed actions +
  reducer separation. Keep as a flag, not the default.

The `dispatch`-closure pattern should be retired (or remain only
under a `--strict-flux` flag for hosts that genuinely want it).

---

### 1.3 Flutter

**(a) What `mosaic-emit-flutter` emits today.** Mirrors React. Callback
props: `onChanged: (value) => dispatch(SomeName.onChange(value: value))`.
**Action shape is a Dart `sealed class NameEvent`** with subclass-per-
emit. Component is a `StatelessWidget` that takes `final void
Function(NameEvent) dispatch` via constructor. No ChangeNotifier or
ValueListenable generated.

**(b) Native Flutter idiom.** Three camps, all popular:

1. **`setState` + `StatefulWidget`** — for component-local state.
   Idiomatic for the smallest cases.
2. **`ChangeNotifier` + `ListenableBuilder` / Provider** — for
   medium cross-widget state. Bundled with Flutter (no library).
3. **BLoC** (`flutter_bloc` package) — for typed-action-and-state
   flows. Sealed event class, sealed state class, BLoC class that
   maps events → states. The closest mainstream library to UI33's
   reducer-with-actions model.

For a spreadsheet-like Grid, BLoC is the natural fit; for simpler
components, ChangeNotifier suffices.

**(c) Blessed libraries.** **flutter_bloc** for typed-flow apps,
**provider** (or built-in `ChangeNotifier`) for simpler state,
**Riverpod** for app-wide reactive composition. Bloc is the closest
analogue to UI33's mental model — same sealed actions, same
reducer-shaped state machine.

**(d) UI33 implication.** Today's `dispatch`-via-callback pattern is
serviceable but unidiomatic. Idiomatic UI33 emission would produce:

- A `class GridCore extends ChangeNotifier` (or `Bloc<GridEvent,
  GridState>` under a flag).
- The View widget consumes via `ListenableBuilder(listenable: core,
  ...)` (ChangeNotifier mode) or `BlocBuilder<GridCore, GridState>`
  (Bloc mode).
- The action enum (sealed class) is generated AND a
  `core.navigate(row, col)` method on the core. Hosts can dispatch
  either way.

Default to ChangeNotifier (simpler, no external dependency); offer
BLoC mode under `--flutter-style=bloc` for hosts that want it.

---

### 1.4 Qt QML

**(a) What `mosaic-emit-qt` emits today.** Pure QML declarative
output. Slots are `property <type> name`; emits become QML signal
declarations: `signal navigate(row: int, col: int)`. Inside event
handlers, `emit navigate(selectedRow, selectedCol)` is called.
Host attaches handlers via `onNavigate: { /* JS */ }` syntax. No
backing C++ class generated.

**(b) Native Qt idiom.** QML's blessed pattern is **signals and slots
on a QObject singleton** with `Q_PROPERTY` for state and
`Q_INVOKABLE` methods for actions:

```cpp
class GridCore : public QObject {
    Q_OBJECT
    Q_PROPERTY(int selectedRow READ selectedRow NOTIFY selectedRowChanged)
    Q_PROPERTY(int selectedCol READ selectedCol NOTIFY selectedColChanged)
public slots:
    Q_INVOKABLE void navigate(int row, int col);
    Q_INVOKABLE void editCommit();
signals:
    void selectedRowChanged();
    void selectedColChanged();
};
```

…registered with `qmlRegisterSingletonType` so QML can `import
GridCore 1.0 as Core` and call `Core.navigate(r, c)`. QML's
declarative bindings (`text: Core.selectedRow`) automatically
re-evaluate on `NOTIFY` signals. Two-way data flow is *the platform
default*.

**(c) Blessed library.** None external — Qt's own `QObject` + `QML`
property bindings are the framework's answer. **CMake + qmake** for
build; **Qt 6 QML** for newer codebases.

**(d) UI33 implication.** Today's pure-QML emission is a fragment;
it can't express "this slot is computed from these other slots" or
"this action mutates state" without a backing C++ class. Idiomatic
UI33 emission would produce:

- A C++ `class GridCore : public QObject` with `Q_PROPERTY` per state
  slot, `NOTIFY` signal per slot, `Q_INVOKABLE` method per action.
- A `qmlRegisterSingletonType` registration in the project shell.
- The QML view binds to `Core.selectedRow` directly, no `dispatch`
  function involved.

This is a meaningful expansion of what the Qt emitter does today —
adds C++ generation alongside QML.

---

### 1.5 HTML (static)

**(a) What `mosaic-emit-html` emits today.** Intentionally minimal.
Static HTML fragment with `data-on-<event>="<emit-name>"` hydration
markers; **no runtime dispatch mechanism**. The host is responsible
for writing a hydrator that walks the DOM, attaches
`addEventListener`s, and dispatches. Slot values use
Handlebars-style `{{slotName}}` template variables, server-rendered.

**(b) Native HTML idiom.** There is no "blessed" pattern — vanilla
HTML/JS apps span everything from raw `addEventListener` to small
libraries like `htmx` (server-rendered HTML over the wire) or
`Alpine.js` (declarative reactive directives). All three are common.

For Mosaic's static-HTML demos: the realistic shape is a small
hand-rolled JS module per project — `const gridCore = createGridCore();
document.addEventListener('click', delegateToCore);`. The host wires
events with delegation.

**(c) Blessed library.** None canonical. If forced to pick, **htmx**
for server-driven HTML (small JS surface), **Alpine.js** for client-
side reactive bits. Neither is as established as React/SwiftUI etc.

**(d) UI33 implication.** UI33 should emit a small **plain ES
module** that exposes the core:

```js
// cores/grid.js
export function createGridCore(initial) {
    const state = { ...defaults, ...initial };
    const target = new EventTarget();
    function navigate(row, col) { /* mutate */ target.dispatchEvent(new Event('change')); }
    return { state, navigate, editCommit, ..., subscribe: (fn) => target.addEventListener('change', fn) };
}
```

The HTML hydrator (also emitted) walks `data-on-*` attributes and
calls `core.navigate(...)` on appropriate events. Host writes none
of this — the project shell wires it.

This is the simplest case but also the **least standardized**, and
the survey suggests the right answer is to be opinionated: pick the
plain-module + EventTarget pattern and ship it.

---

### 1.6 Web Components

**(a) What `mosaic-emit-webcomponent` emits today.** A `class
extends HTMLElement` with shadow DOM. Inline event handlers call
`this.dispatch(eventObj)`, which fires `new CustomEvent("mosaic:" +
event.type, {detail: event, bubbles: true, composed: true})`. Host
listens via `addEventListener("mosaic:navigate", e => ...)` on the
element or any ancestor. Per-emit programmatic methods (`onClick()`)
are also generated.

**(b) Native Web Components idiom.** Web Components proper has no
opinion on state management — it's just the element API. In practice
the ecosystem uses **Lit** (`lit-html` + `ReactiveElement`) which
gives you:

- `@property()` decorators for reactive properties (re-renders on
  change)
- `dispatchEvent(new CustomEvent(...))` for outgoing events (exactly
  what we emit today)
- Imperative method calls for actions on the element

State containers come from outside: Redux, Zustand, or just a plain
module the element subscribes to.

**(c) Blessed library.** **Lit** for the element layer; **state lives
elsewhere** (any of the JS-ecosystem solutions). The element is
explicitly a *view layer*.

**(d) UI33 implication.** The right shape is:

- The core is a **plain ES module** (same as §1.5 — share the
  generated `cores/grid.js`).
- The Mosaic-emitted custom element holds a reference to the core
  instance, subscribes to its changes, and dispatches CustomEvents
  outward when the host needs to react.
- An `attributeChangedCallback` on the element accepts config
  attributes that map to Layer-3 state defaults (§3.5.3 of UI33).

The core file is **shared between HTML and Web Component emission**
— same module, different mounting strategy. This is a nice
consolidation opportunity.

---

### 1.7 XAML (WinUI 3 / WPF)

**(a) What `mosaic-emit-xaml` emits today.** A WinUI 3 component
with `EventHandler<T> Dispatch` on the code-behind class, fired via
`RaiseDispatch(ev)`. **Action shape is a sealed C# record union** —
`public abstract record GridEvent;` with per-emit `public sealed
record Navigate(int row, int col) : GridEvent;` subclasses. Host
subscribes with `grid.Dispatch += (s, e) => switch (e) { ... }`.
Slot values are `DependencyProperty`s wired via `{x:Bind}`.

**(b) Native XAML/WinUI/WPF idiom.** **MVVM** — Model-View-ViewModel.
The ViewModel holds state and exposes actions as `ICommand`
instances. Properties implement `INotifyPropertyChanged`. The View
binds via `{Binding}` (WPF) or `{x:Bind}` (WinUI 3). This is *the*
canonical pattern; almost all production XAML code follows it.

```csharp
public partial class GridCore : ObservableObject {
    [ObservableProperty] private int selectedRow;
    [ObservableProperty] private int selectedCol;
    [RelayCommand] private void Navigate(NavigateArgs args) { /* mutate */ }
    [RelayCommand] private void EditCommit() { /* mutate */ }
}
```

The `[ObservableProperty]` and `[RelayCommand]` attributes are
**source-generators from `CommunityToolkit.Mvvm`** that produce the
INPC + ICommand boilerplate at compile time. This has effectively
replaced hand-written ViewModels in modern XAML.

**(c) Blessed library.** **CommunityToolkit.Mvvm** — the de facto
standard. Ships with `ObservableObject`, `[ObservableProperty]`,
`[RelayCommand]`, `Messenger` (pub-sub), etc.

**(d) UI33 implication.** Today's `EventHandler<T> Dispatch` pattern
is awkward for XAML hosts — it makes them write a `switch` on event
records when MVVM gives you typed `ICommand`s for free. Idiomatic
UI33 emission would produce:

- A `partial class GridCore : ObservableObject` with
  `[ObservableProperty]` per state slot.
- `[RelayCommand]` per action (yields a `NavigateCommand` property
  the View binds to `Command="{x:Bind Core.NavigateCommand}"`).
- The View's `DataContext` is set to the core; bindings flow
  naturally.

This is a substantial reshape of the current XAML emitter — but it
matches what every XAML developer expects to see.

---

## 2. Cross-cutting comparison

### 2.1 What each backend emits today vs. what's idiomatic

| Backend       | Today                                  | Idiomatic                                              | Mismatch?  |
|---------------|----------------------------------------|--------------------------------------------------------|------------|
| React         | JSX onChange/onClick + dispatch prop   | `useReducer` or `useGridCore()` hook + actions object | Minor      |
| SwiftUI       | `dispatch` closure prop + View struct  | `@Observable class GridCore` + method calls           | **Major**  |
| Flutter       | callback props + dispatch fn           | `ChangeNotifier` or `Bloc` + method calls             | **Major**  |
| Qt QML        | declarative QML signals, no backing C++| `QObject` singleton + `Q_PROPERTY` + `Q_INVOKABLE`    | **Major**  |
| HTML          | static + data-on-* hydration markers   | plain ES module + `EventTarget`                       | Moderate   |
| Web Component | CustomEvent via shadow DOM             | Lit + same plain ES module as HTML                    | Minor      |
| XAML          | `EventHandler<T> Dispatch` + records   | MVVM `ObservableObject` + `[RelayCommand]`            | **Major**  |

**Half the backends have a major idiom mismatch.** Today's
`dispatch`-closure pattern is React-shaped and was bolted onto the
others. Cores under UI33 are an opportunity to fix this.

### 2.2 Native action representation

| Backend       | Native action shape                                                       |
|---------------|---------------------------------------------------------------------------|
| React         | TS discriminated union: `{type: "navigate"; row: number; col: number}`   |
| SwiftUI       | Swift enum + associated values: `enum Action { case navigate(row:Int,…)}`. *Idiomatic alternative:* method calls (no action type at all). |
| Flutter       | Dart sealed class hierarchy. *Idiomatic alternative:* method calls.       |
| Qt            | Typed signal: `signal navigate(int row, int col)`. *Or:* `Q_INVOKABLE`-method call (no signal). |
| HTML          | `CustomEvent` with detail payload (if event-based) OR plain function call |
| Web Component | `CustomEvent` (outbound) OR method call (inbound)                         |
| XAML          | Sealed C# record (event-style) OR `ICommand`-with-parameter (MVVM-style)  |

The pattern: **every backend has both an "event/action" shape AND
a "method call" shape**, and the *more idiomatic* shape varies.
SwiftUI / Qt / XAML lean method-call (because their reactive systems
make state-mutation-then-re-render automatic). React / Flutter /
HTML / WebComponent lean event/action (because they're more
explicit-update-style).

### 2.3 State container

| Backend       | Idiomatic state container                          |
|---------------|----------------------------------------------------|
| React         | `useReducer`, Zustand store, Redux Toolkit slice   |
| SwiftUI       | `@Observable class` (iOS 17+); else `ObservableObject` |
| Flutter       | `ChangeNotifier`, BLoC, Riverpod provider          |
| Qt            | `QObject` + `Q_PROPERTY` + `NOTIFY`                |
| HTML          | Plain ES module + `EventTarget`                    |
| Web Component | Same as HTML (Lit `@property` for view-local only) |
| XAML          | `ObservableObject` (MVVM Toolkit)                  |

### 2.4 Async / effects

| Backend       | Async primitive                                    |
|---------------|----------------------------------------------------|
| React         | `useEffect`, async/await, Promises                 |
| SwiftUI       | `Task { await ... }` from `.task` modifier         |
| Flutter       | `async`/`await`, `Stream`, `Future`                |
| Qt            | `QFuture`, signal-callback chains, `async` keyword |
| HTML          | Promises, async/await                              |
| Web Component | Same                                               |
| XAML          | `async`/`await`, `Task<T>`                         |

All seven support host-side async. **None need it inside the
`.core` DSL** — keeping `.core` synchronous (per UI33 §3.6) means
async happens at the host extension-point boundary, which all seven
backends model uniformly.

### 2.5 Two-way binding

| Backend       | Two-way binding default?               | Implication                                          |
|---------------|----------------------------------------|------------------------------------------------------|
| React         | No (explicit `value`/`onChange`)       | Dispatcher emits both directions explicitly          |
| SwiftUI       | Yes (`@Binding`, `@Bindable`)          | Can emit `core.foo` reads/writes directly            |
| Flutter       | No                                     | Same as React                                        |
| Qt            | Yes (QML property bindings)            | Can emit `text: Core.foo`                            |
| HTML          | No                                     | Hydrator handles both directions                     |
| Web Component | No (CustomEvent + property mutation)   | Same as HTML                                         |
| XAML          | Yes (`{x:Bind Mode=TwoWay}`)           | Can emit two-way binding directly                    |

SwiftUI, Qt, XAML expect two-way binding. React, Flutter, HTML,
Web Component are unidirectional. **The dispatcher emitter has to
know this per backend** and emit accordingly.

---

## 3. Implications for UI33

### 3.1 The big realization

**UI33's "one dispatcher contract for all backends" is the wrong
framing.** Three of the seven backends (SwiftUI, Qt, XAML) are
*reactive* — they expect state mutation + automatic view refresh,
with method-call action shape and two-way bindings. Four
(React-medium, Flutter, HTML, WebComponent) are *event-flow*-style
— explicit dispatch, unidirectional flow.

Trying to force a uniform `dispatch(action)` API onto SwiftUI / Qt /
XAML produces unidiomatic code that fights the platform. The
realization: **`.core` should be the IR, but the emitted shape
should match the platform's blessed pattern, not a one-size-fits-all
dispatcher contract.**

### 3.2 Two emitter modes per backend

The cleanest answer is: **each backend's core emitter supports two
modes**, selectable via `--<backend>-style=...`:

| Mode             | Suits                                  | Backends where this is default  |
|------------------|----------------------------------------|---------------------------------|
| **reactive**     | platforms with built-in reactivity     | SwiftUI, Qt, XAML               |
| **event-flow**   | platforms with explicit dispatch       | React, Flutter, HTML, WebComp  |

Both modes are equivalent under the hood — the same `.core` source
lowers to either. The dispatcher decides which one to emit based on
backend default (configurable per project).

### 3.3 Concrete per-backend "blessed shape" the emitter should produce

| Backend       | Blessed shape (UI33 v0.1.0)                                                  |
|---------------|------------------------------------------------------------------------------|
| React         | `useGridCore({...})` hook → `{state, actions: {navigate, editCommit, ...}}` |
| SwiftUI       | `@Observable final class GridCore { var state; func navigate(...); ... }`    |
| Flutter       | `class GridCore extends ChangeNotifier { ... void navigate(...); ... }`      |
| Qt            | C++ `class GridCore : public QObject` w/ `Q_PROPERTY`+`NOTIFY`+`Q_INVOKABLE`, registered as QML singleton |
| HTML          | Plain ES module `createGridCore(initial)` returning `{state, navigate, ..., subscribe}` |
| Web Component | Same ES module as HTML; emitted custom element holds a reference to the core |
| XAML          | `partial class GridCore : ObservableObject` with `[ObservableProperty]` + `[RelayCommand]` |

### 3.4 The dispatcher's role narrows

Once each backend gets its native shape, the **dispatcher's job
shrinks to: wire generated Mosaic component event handlers to the
appropriate `core.<method>(args)` call.** The dispatcher is no
longer a runtime object that routes typed actions — it's a
compile-time *binding generator* that emits the correct method
call per backend.

This is a meaningful simplification of UI33. The `.disp` file's job
becomes:

```
// .disp file
dispatcher VisiCalc {
    uses core mosaic-core-grid as grid
    mounts component Grid from "./Grid.mil"

    // The auto-derivation says:
    //   Grid.onNavigate (component emit)
    //     → grid.navigate (core method)
    //
    // and the per-backend emitter writes either:
    //   React:     core.actions.navigate(row, col)
    //   SwiftUI:   core.navigate(row: row, col: col)
    //   Flutter:   core.navigate(row, col)
    //   Qt:        Core.navigate(row, col)
    //   HTML:      core.navigate(row, col)
    //   WebComp:   core.navigate(row, col)
    //   XAML:      core.NavigateCommand.Execute(new NavigateArgs(row, col))
    // The `dispatch` function disappears from the emitted code.
}
```

### 3.5 Override layers map naturally to native idioms

| Layer | Mechanism                | React                      | SwiftUI                    | Flutter                    | Qt                         | XAML                       |
|-------|--------------------------|----------------------------|----------------------------|----------------------------|----------------------------|----------------------------|
| 1     | Named extension point    | Function passed to hook     | Closure property           | Function field             | Lambda / signal connection | Delegate property          |
| 2     | Action override          | Reducer slice override      | Subclass with override     | Subclass with @override    | Subclass overriding slot   | Method override            |
| 3     | State default            | Initial argument to hook    | Init parameter             | Constructor parameter      | QML import params          | Constructor parameter      |

All three layers map onto patterns developers already know in each
ecosystem. Layer 1 in particular maps onto each language's natural
"pass me a callback" idiom — closures in Swift, function fields in
Dart, lambdas in Qt JS, delegates in C#.

### 3.6 Action shape: still a tagged union internally, native at the boundary

The `.core` DSL's action declaration (`action navigate(row: number,
col: number)`) lowers to **two artefacts per backend**:

1. A method on the core class/object (`navigate(row, col)`).
2. *Optionally* an action enum/sealed class **if the host opts into
   it** for instrumentation (action logging, replay, time-travel
   debugging — the UI33-D-* follow-up).

By default, only the method exists. Hosts that want the audit trail
flip `--emit-action-log` and get both.

### 3.7 What this changes about UI33 (specifically)

| Section of UI33         | Change                                                                        |
|-------------------------|-------------------------------------------------------------------------------|
| §2 architecture diagram | Dispatcher is compile-time, not a runtime layer. The diagram should show this. |
| §3 `.core` DSL          | Largely unchanged — still the right IR.                                       |
| §4 `.disp` DSL          | Job narrows from "route typed actions" to "bind component emits to core methods". Simplifies the grammar. |
| §3.3 per-backend table  | Replace with the §3.3 table above (idiomatic shapes per backend).             |
| §3.5 override layers    | Add the per-backend mapping above to §3.5.                                    |
| §8 implementation plan  | Each `UI33-E-X-1` becomes "emit core class/module in idiomatic shape" rather than "emit generic state container". |
| §9 risks                | Add: "Two emitter modes (reactive vs event-flow) per backend doubles emitter complexity; can we get away with one shape per backend?" |

---

## 4. Open questions for the next UI33 amendment

1. **One mode per backend or two?** Are we OK shipping `SwiftUI =
   @Observable only` and `XAML = MVVM only`, or do we want a
   `--style=flux` flag for hosts that want unidirectional flow on
   reactive platforms?
2. **Web Component view layer: use Lit?** Lit is the de facto Web
   Components library. Should the emitter target Lit explicitly
   (depend on it as a transitive package) or stay zero-dep?
3. **Qt: introduce C++ generation?** The Qt emitter is QML-only
   today. Adding a C++ class generator is a significant expansion.
   Alternative: keep Qt pure-QML and have the core live as a QML
   singleton with JavaScript-defined methods (less idiomatic but
   zero-C++).
4. **XAML: depend on CommunityToolkit.Mvvm?** It's the de facto
   standard but it IS a NuGet dependency. Alternative: emit raw
   INPC + ICommand boilerplate (more code, no dependency).
5. **HTML: be opinionated about the hydrator?** UI33 should ship one
   reference hydrator pattern (the plain-ES-module + EventTarget
   shape in §1.5d), or stay un-opinionated and let each project
   write its own.
6. **Two-way bindings: explicit in `.mll` or implicit per backend?**
   When a `.mll` writes `value: slot: edit-content` + `onChange:
   emit: onFormulaChange`, that's unidirectional. SwiftUI/Qt/XAML
   could lower the same `.mll` to two-way binding. Should they? Or
   should two-way be a separate `binding: two-way slot: ...` form?

---

*End of survey. Recommend reading §3.1–3.4 even if skipping
everything else — those are the design-affecting findings.*
