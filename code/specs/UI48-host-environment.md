# UI48 — Host environment: runtime viewport, input modality, and variant selection

**Status:** In progress — ENV1's runtime half implemented in `mosaic-app-runtime` (§7.1); ENV2 and ENV3 on SwiftUI (§7.2)
**Layer:** UI / standard Mosaic app ABI
**Depends on:** UI29 (primitive kernel), UI30 (multi-layout pipelines), UI38
(native application runtime), `mosaic-app-runtime`, `mosaic-app-capi`
**Completes:** UI30's unbuilt **ML4** / **ML5**
**Counterpart of:** UI47 (host *capability effects* — the outbound direction)
**Tracked by:** [#14003](https://github.com/adhithyan15/coding-adventures/issues/14003)
**First consumer:** #13692 (TaskApp compact-window shell)

---

## 1. The question this settles

A Mosaic app cannot respond to its own runtime environment. It cannot know how
wide its window is, whether it is being touched or clicked, which way a device
is held, or whether the user asked for reduced motion.

It is not that the stack has no environment concept. It has one — and it is
frozen at startup. `StartContext` already carries `locale`, `color_scheme`,
`text_scale`, and `platform`. What is missing is a **change channel**: nothing
tells the app when any of it moves, and four of the axes that matter most are
not in the struct at all.

UI47 gave the app a way to ask the host to *do* something. This is the inbound
half: a way for the host to tell the app what it *is*, and to keep telling it.

---

## 2. What is actually there today

Every claim here was verified against `main` at the time of writing, with the
implemented backends as a control rather than by reading the specs alone.

**mosstyle has no media queries.** The authored style vocabulary is a flat
property set — `width`, `min-width`, `max-width`, `flex-*`, and so on. There is
no `@media`, no breakpoint construct, and no conditional block. A style cannot
vary on anything but the theme axis.

**`--variant` is compile-time.** `mosaic-compile --variant touch` resolves
`<Component>.touch.mll` at build time, falling back to the bare
`<Component>.mll`. UI30 §4 calls this "the multi-layout equivalent of CSS's
`@media` cascade", and it is — but it is a *file selection*, decided before the
program runs. A window that changes size after launch cannot be served by it.

**Runtime selection was explicitly deferred.** UI30 §6 says, verbatim, that it
"does *not* prescribe how the host picks a variant at runtime — that's outside
the compiler's scope", and lists **ML4 — runtime LayoutSwitch toolkit
component** as a future cycle. `LayoutSwitch` appears nowhere in the repository
except that sentence.

**No backend observes anything.** None of the nine `mosaic-emit-*` crates
contains `matchMedia`, `ResizeObserver`, `MediaQuery`, `WindowSizeClass`,
`horizontalSizeClass`, `SizeChanged`, or `resizeEvent`. `mosaic-app-runtime`
has no viewport, size-class, pointer, or orientation concept. This is a total
gap, not a partial one.

**The one runtime conditional is `If ( when: slot: … )`**, which branches on an
app data slot. That is the mechanism an app would have to abuse to fake
responsiveness — and §3 explains why it must not.

**But two pieces of the answer already exist**, which is why this spec proposes
an extension rather than a new subsystem:

- **`StartContext` is already an environment struct.** `protocol_version`,
  `locale`, `color_scheme`, `text_scale`, `platform`, `restored_snapshot`. It is
  delivered once, at `Runtime::start`, and never updated.
- **`Event` is already the generic host→app channel.** `{ protocol_version,
  sequence, name, payload }` — a name and a JSON payload, dispatched through
  `Runtime::dispatch`. It is not bound to a control: its own doc comment calls
  it "a semantic UI event **or a completed host effect**", so it is already the
  transport UI47 uses for effect completion. A host can originate one for any
  reason.

---

## 3. Why not userland conditionals

The obvious workaround is for each app to declare a `compact` slot, have its
host set it from the window width, and branch the layout on it.

**UI30 already considered and rejected this**, in its own words:

> Userland conditional rendering (`If sizeClass == compact { Column } Else
> { Row }`) is possible in theory but rapidly bloats components and loses the
> "declarative layout per form factor" intent.

Three further problems, beyond bloat:

1. **It is per-app, so it is nine reinventions.** Every app would define its own
   slot name, its own breakpoint, and its own host-side observation code, once
   per backend. The mechanism belongs to the kernel precisely because every app
   needs it and none of them should own it.
2. **It only reaches layout.** A slot cannot vary a *style*, so touch-sized tap
   targets — the actual reason UI30 wanted a touch variant — remain
   unexpressible no matter how many slots are added.
3. **It puts a presentation concern in the data contract.** The `.mil` is what
   data flows in and out. Window width is not data; it is the environment the
   data is being rendered into. UI30 keeps the `.mil` singular across form
   factors for exactly this reason.

This spec therefore does **not** add a general "environment slot" that layouts
branch on ad hoc. It makes the environment a first-class kernel input, and
routes it to the mechanism UI30 already chose: variants.

---

## 4. The environment vocabulary

A closed, kernel-owned set. Closed because every backend must be able to answer
every question, and because an open set becomes an untestable matrix.

| Axis | Values | Meaning |
| --- | --- | --- |
| `size-class` | `compact` \| `regular` \| `expanded` | Available width bucket |
| `pointer` | `coarse` \| `fine` \| `none` | Primary pointing device precision |
| `hover` | `hover` \| `none` | Whether hover affordances are reachable |
| `orientation` | `portrait` \| `landscape` | Window, not device |
| `color-scheme` | `light` \| `dark` | Already an authored axis; unified here |
| `reduced-motion` | `reduce` \| `no-preference` | Accessibility preference |

**Buckets, not pixels.** `size-class` is deliberately not a number. A pixel
threshold is a web idea; Compose has `WindowSizeClass`, SwiftUI has
`horizontalSizeClass`, and a TV host has neither. Naming buckets lets every
backend map its native concept onto the same vocabulary instead of emulating
CSS. The default thresholds (`compact` < 600, `regular` < 1024, `expanded`
above, in density-independent units) are a *host-side default*, overridable per
host, not part of the authored contract.

**`hover` is separate from `pointer` on purpose.** They are not the same
question, and treating them as one is a well-known source of broken touch UIs:
a stylus is fine-grained but cannot hover; a TV remote is neither.

**`color-scheme` unifies an axis that already exists.** Theme is currently a
whole-component swap (`.light.msl` / `.dark.msl`, two emitted components, host
picks). That works, and this spec does not change it. It is listed here so the
vocabulary is complete and so a future cycle can fold the two mechanisms
together rather than leaving theme as a permanent special case.

---

## 5. How the environment reaches the app

Three layers, each with one job.

### 5.1 Observation — per backend, generated

Each emitter generates the observer natural to its platform. This is the part
that cannot be shared, and the reason this belongs in the emitters rather than
in a toolkit component:

| Backend | Observes with |
| --- | --- |
| `react`, `html`, `webcomponent` | `matchMedia` for pointer/hover/scheme/motion; `ResizeObserver` on the mount for size-class |
| `qt` | `QWidget::resizeEvent`; `QGuiApplication::styleHints()` |
| `flutter` | `MediaQuery.of(context)` |
| `compose` | `calculateWindowSizeClass`; `LocalConfiguration` |
| `swiftui` | `horizontalSizeClass`; `GeometryReader`; `accessibilityReduceMotion` |
| `xaml` | `SizeChanged`; `PointerDeviceType`; `UISettings` |
| `paint` | Fixed — a snapshot backend *declares* its environment at render time |

### 5.2 Transport — events, which already exist

**Events are the primitive.** The environment is not a new channel; it is
`StartContext` extended with the missing axes, plus one reserved event that
redelivers it whenever it changes:

- **Initial value** — the axes in §4 join `StartContext`, beside the
  `color_scheme`, `text_scale`, `locale`, and `platform` already there. An app
  therefore knows its environment *before* first render and cannot flash a
  desktop shell into a phone window.
- **Change** — a single reserved event, `environmentChanged`, whose payload is
  the whole environment. Dispatched through the existing `Runtime::dispatch`;
  no new ABI surface, and it versions with `protocol_version` like everything
  else.

Three consequences of choosing events, each of which is a reason rather than an
accident:

1. **One coalesced event, not one per axis.** Rotating a device changes
   orientation and size-class together; a stylus being set down can change
   pointer and hover together. Per-axis events would expose intermediate states
   that never existed and force N re-renders. Apple's trait collection changes
   atomically for the same reason. The payload is the whole struct.
2. **Emitted on bucket change, never per pixel.** A resize fires continuously;
   `size-class` does not. Because §4 is a coarse vocabulary, the host observes
   at native frequency and dispatches only when a *bucket* flips — typically a
   handful of events in a session rather than sixty a second crossing the ABI.
   The coarse vocabulary is what makes the event channel affordable.
3. **A host that never dispatches one keeps today's behavior exactly.** The
   defaults are `regular`/`fine`/`hover`/`landscape`/`no-preference`. This is
   what makes the change additive across seven shipped bindings rather than a
   break.

### 5.3 Selection — the kernel picks the variant

The compiled artifact carries every authored variant (UI30's ML5). A
kernel-generated selector maps environment to variant and re-renders when the
environment changes. Authors keep writing `<Component>.touch.mll` exactly as
UI30 specified; what changes is that the choice is no longer frozen at build
time.

Resolution stays UI30's fallback chain: requested variant, then the bare
default. Nothing is required to author every variant.

---

## 5.4 Prior art, and the container this spec is missing

The vocabulary above is not invented. It is close to what the platforms already
converged on, and the divergences are worth naming because they are where the
mapping costs land.

| Platform | Size model | Mechanism |
| --- | --- | --- |
| Apple UIKit / SwiftUI | Semantic buckets — `.compact` / `.regular` | `UITraitCollection`, `registerForTraitChanges`, `@Environment(\.horizontalSizeClass)` |
| Android / Compose | Semantic buckets — Compact / Medium / Expanded | `WindowSizeClass` |
| Windows WinUI | **Numeric thresholds** | `VisualStateManager` + `AdaptiveTrigger MinWindowWidth` |
| Web / CSS | Numeric, plus separate `pointer:` / `hover:` | Media queries, container queries |
| Flutter | Numeric | `MediaQuery`, `LayoutBuilder` |
| Qt | Numeric | `resizeEvent`, `QStyleHints` |

Two consequences:

- **`size-class` as buckets follows Apple and Android and taxes the other
  four.** Windows, Web, Flutter, and Qt all reason in numbers, so they map down
  into buckets and lose the ability to express "at exactly 900, do X". That is
  the cost of §8's open question 1, stated plainly rather than hidden.
- **Separating `hover` from `pointer` follows CSS Media Queries Level 4**,
  which splits them for precisely the stylus-and-TV reason given in §4.

**What this spec is missing.** Neither Apple nor Windows answers resize
*primarily* with a query. Both ship an adaptive **container control** that
already encapsulates the behavior:

| Platform | Control | Behavior |
| --- | --- | --- |
| Apple | `UISplitViewController` / `NavigationSplitView` | Side-by-side at regular, collapses to a navigation stack at compact |
| Windows | `NavigationView` (`PaneDisplayMode="Auto"`) | Expanded pane → compact icon rail → overlay, at roughly 640 and 1008 |
| Android | `NavigationSuiteScaffold` | Drawer / rail / bottom bar by size class |

Mosaic has no container primitive to lower these onto. The kernel inventory is
`HostButton`, `HostCheckbox`, `HostDialog`, `HostDraggable`, `HostDropTarget`,
`HostInput`, `HostLink`, `HostNumberInput`, `HostProgressRing`, `HostRadio`,
`HostScroll`, `HostSlider`, `HostSurface`, `HostSwitch`, `HostTable` (+ its
parts), and `HostTooltip` — leaves and one table.

So this spec, on its own, would have every backend hand-roll the adaptive shell:
a real `NavigationView` on Windows replaced by two containers and a visibility
branch. That is exactly what
[#12017](https://github.com/adhithyan15/coding-adventures/issues/12017) —
"make Mosaic emit real native components" — exists to prevent, and TaskApp's
rail is precisely the split-view/navigation-pane pattern these controls own.

**The environment is necessary but not sufficient.** A companion primitive —
provisionally `HostNavigationSplit` — should lower to `UISplitViewController`,
`NavigationView`, and `NavigationSuiteScaffold`, consuming this environment
rather than reimplementing it. It is deliberately not specified here: it is a
kernel primitive under UI29's rules and needs its own spec and its own
per-backend degradation story. That spec is now written:
[UI29-6](UI29-6-host-navigation-split.md) (#15481). Variant selection (§5.3) remains the general
mechanism for everything that is *not* a standard navigation shell.

---

## 5.5 What "the generated code absorbs the quirks" can and cannot mean

Standardizing on events puts every platform difference in one place: the
emitter. That is the right place — an emitter already knows its platform, and
`resizeEvent` versus `matchMedia` versus `WindowSizeClass` is exactly the kind
of difference emitted code should hide. But the claim has a hard boundary, and
pretending otherwise is how a portable abstraction turns into a leaky one.

**Mechanical quirks — absorbed.** These are differences in *how you learn* a
fact both platforms agree exists:

- Observation API — `resizeEvent`, `matchMedia` + `ResizeObserver`,
  `MediaQuery`, `WindowSizeClass`, `horizontalSizeClass`, `SizeChanged`.
- Coalescing and debounce policy, which differs per toolkit.
- Units — density-independent pixels, points, CSS pixels, physical pixels.
- Synthesis where an axis is missing. macOS never adopted size classes, so the
  Apple emitter derives buckets from window width; the app cannot tell.

**Semantic divergence — not absorbed, and must not be.** These are cases where
platforms disagree about what *exists*, and flattening them produces an app
that is wrong everywhere rather than portable:

- **Navigation models.** Android's system back button and iOS's interactive
  swipe-back are not the same gesture with different plumbing; they imply
  different information architecture. No event shape reconciles them.
- **"Touch" is not one thing.** A Windows 2-in-1 in tablet mode, an iPad with a
  trackpad attached, and a phone are three different combinations of
  `pointer`/`hover`, which is precisely why §4 keeps them as separate axes.
- **Window models.** Tiling, snapping, split-screen, and Stage Manager change
  what a "resize" means and how often it happens.

The rule this spec adopts: **an emitter may synthesize a value in the closed
vocabulary; it may not invent vocabulary, and it may not silently paper over a
platform that cannot answer.** Where a backend genuinely cannot supply an axis,
it reports the documented default and emits a degradation — the same mechanism
the kernel already uses when a backend cannot honor an authored construct — so
the gap is visible in the build rather than discovered by a user.

### 5.6 Where events are the wrong answer

Two limits, both consequences of events being a *transport* rather than a
semantics.

**Events cannot reach styles.** mosstyle bakes its values into each emitted
component's inline styles; there is no runtime style layer to update. An event
can change a slot, and a slot can gate a layout branch, but nothing can restyle
a live tree. So the 44×44 touch target that motivated UI30's touch variant is
still unreachable by events alone. This is why §5.3's variant selection stays in
the design: the event is the *signal*, and swapping the emitted component is the
*mechanism*. This is the same shape the light/dark theme swap already uses, and
it is why §4 lists `color-scheme` — the two mechanisms should converge.

**Native adaptive containers should not round-trip.** A `UISplitViewController`
or a WinUI `NavigationView` adapts internally, in the platform's own layout
pass. Routing its behavior through an event — resize, dispatch, adapter state,
new props, re-render — would be slower, would jank against the platform's own
animation, and would replace a real native control with a hand-rolled
imitation. For the container in §5.4, the correct amount of environment
plumbing is **none**: the control already knows. Events serve everything that is
*not* a standard native adaptive control.

---

## 6. What this deliberately does not do

- **It does not add `If ( when: env: … )`.** §3 is the reasoning. If a genuine
  need appears for a conditional too small to justify a variant file, it should
  arrive as an amendment with the motivating case, not be speculatively
  included here.
- **It does not change theme.** The `.light`/`.dark` component swap ships and
  works; folding it into this vocabulary is a later cycle.
- **It does not define gestures.** Swipe, pinch, and long-press are input
  *events*, not environment. UI35's drag primitives are the existing seam; touch
  gestures belong with them, not here.
- **It does not make `paint` responsive.** A snapshot backend declares its
  environment; that is the whole of its participation.

---

## 7. Implementation plan

Sliced so each lands independently and provably.

- **ENV1 — extend `StartContext`, add `environmentChanged`.** The §4 axes join
  the struct beside `color_scheme`/`text_scale`/`platform`, with defaults, plus
  the one reserved event through the existing `Runtime::dispatch`. No emitter
  changes and no new ABI surface; the test that matters is that a host which
  dispatches nothing behaves identically to today.
- **ENV2 — artifact carries every variant** (UI30's ML5). Compile all authored
  variants into one artifact with stable per-variant names.
- **ENV3 — selection.** The kernel selector: environment in, variant out, with
  UI30's fallback chain. Unit-testable with no backend at all.
- **ENV4..N — one backend per PR.** Generate the observer, wire it to the
  runtime, and add an acceptance test that *resizes* and asserts the swap. A
  gate that only renders at one size would pass against a frozen layout, so the
  test must change the environment.
- **ENV-last — TaskApp compact shell (#13692).** Author `TaskApp.compact.mll`
  and delete nothing else. If this spec is right, the app-side change is a new
  layout file and no new slots, no new emits, and no host-specific code.

---

### 7.1 ENV1 as built

Decisions taken while implementing ENV1 in `mosaic-app-runtime`:

- **Wire shape.** The five new axes join `StartContext` flat, beside
  `colorScheme`: `sizeClass`, `pointer`, `hover`, `orientation`,
  `reducedMotion`, in kebab-case values (`"compact"`, `"no-preference"`). Each
  is optional on the wire and defaults to §5.2's
  `regular`/`fine`/`hover`/`landscape`/`no-preference`, so every existing host
  decodes unchanged. In Rust they are one flattened field,
  `StartContext::environment: EnvironmentAxes`, and
  `StartContext::full_environment()` pairs them with the color scheme.
- **The event.** `environmentChanged` (`ENVIRONMENT_CHANGED`) carries the whole
  `Environment` — `colorScheme` plus the five axes, every one required, unknown
  keys ignored so a newer host can add an axis. It is an ordinary event: it
  consumes the next sequence number.
- **The runtime intercepts it; apps opt in.** The runtime decodes the payload
  (an invalid one is refused as `InvalidEnvironment` before the app sees
  anything, consuming nothing) and calls a new trait method,
  `MosaicApp::environment_changed`, instead of `dispatch`. Its default answers
  "no reaction", so every existing app — each of which rejects event names it
  does not know — keeps working when a host starts sending the event.
- **"No reaction" on the wire.** Updates carry whole props, and the runtime
  keeps no copy of them. When the app does not react, the runtime returns an
  update with the **current** revision (not incremented), `props: null`, and
  no effects or announcements. A host treats an update whose revision is not
  newer than the one it rendered as nothing to render. No host sent this event
  before ENV1, so no existing host can receive such an update.
- **The runtime remembers the environment** (`Runtime::environment()`), from
  the start context and each change, for hosts and tests.
- **Reserved name.** `environmentChanged` is the runtime's; a package emitting
  an event of that name would be intercepted. Refusing it in `mosmodel` is a
  follow-up.
### 7.2 ENV2 and ENV3, designed

Written before implementation, from what the pipeline does today (checked on
`main` with Engram, whose `EngramApp.touch.mll` is the only root-level
variant): the artifact builder already compiles every authored variant, and
each emitter writes `<Component>.<variant>.<ext>` beside the default. What it
cannot do is put two variants **in one app**. On SwiftUI both files declare
`struct EngramAppView` and repeat the event enum and its extension, so only
the default is compiled into `Sources/App`.

**ENV2 — one app carries every variant.**

- Each variant's root is a distinct type: `<Component><Variant>View` (the
  variant name in PascalCase: `touch` → `EngramAppTouchView`), and each
  backend's equivalent (a `@Composable` function, a Dart widget class, a QML
  component, a XAML user control — XAML already suffixes its type).
- A variant file carries only what differs: its view. The component's
  interface — its event type, prop accessors — is the same for every variant
  (UI30 §2.2 puts the variant on the layout, never the interface) and is
  emitted once, by the default. Helpers private to a file may repeat.
- The project shell compiles every variant into the app. `--variant` at build
  time (UI30 §3) still builds a single-variant artifact for hosts that want
  one; this is the carry-everything default for project shells only.

**ENV3 — the selector: environment in, variant out.**

Variant names are opaque to the compiler (UI30 §2), so which environment
selects which variant is declared, in the package manifest:

```toml
# First match wins; no match is the default layout.
[[app.layouts]]
variant = "compact"
size-class = "compact"

[[app.layouts]]
variant = "touch"
pointer = "coarse"
```

- Each entry names a variant and the UI48 axes that must all match. An array
  of tables, because its order is TOML's own — a table's key order is not
  something a TOML parser promises. A variant with no rule is never selected
  at run time (it can still be built alone with `--variant`). A rule with no
  axes always matches, so only the last rule may have none.
- Without `[[app.layouts]]`, the conventional names select themselves:
  `compact` for `size-class = compact`, `expanded` for `size-class =
  expanded`, `touch` for `pointer = coarse`, in that order. Any other name
  needs a rule.
- The selector is a pure Rust function in the kernel (`mosaic-package-manifest`
  parses the rules; a `select_variant(environment, rules)` beside it decides),
  unit-tested without a backend. Each backend's shell emits the same rules in
  its own language, generated from the Rust table, so the choice is identical
  everywhere.
- Selection runs on the environment the host observes (ENV4..N): SwiftUI's
  `horizontalSizeClass` and scene phase, Compose's `WindowSizeClass`, the
  web's `matchMedia`. When the chosen variant changes, the shell mounts the
  other root view with the same props; the app's state lives in the runtime,
  so nothing is lost. The same observation is sent to the runtime as
  `environmentChanged` (ENV1), for apps that also want to react in logic.

**Order of work.** ENV2 and ENV3 land together per backend, SwiftUI first
(iPhone and iPad), with the Rust selector and manifest table in the first
PR. The acceptance test changes the environment and asserts the swap, as §7
requires: TaskApp gets `TaskApp.compact.mll`, and the iOS simulator gate
launches it on a phone (compact) and asserts the compact layout's marker.

### 7.3 ENV4 on SwiftUI, designed

ENV3 made SwiftUI *choose* a layout from what it observes. ENV4 makes it
*tell the runtime*, so an app can also react in logic (§5.2), and so the
runtime's `environment()` is the one the user is looking at.

- **Every runtime-backed app observes.** The generated shell wraps its root in
  `MosaicEnvironmentReader` whether or not the package has layout variants;
  the selector is still generated only for packages that have some. A
  sample-props shell (no runtime) does not observe: there is nothing to tell.
- **Reported on bucket change only.** The reader reduces what it sees to the
  six §4 values (`colorScheme`, `sizeClass`, `pointer`, `hover`,
  `orientation`, `reducedMotion`) and reports them with `.task(id:)` keyed on
  those values: once when the window first appears, then only when one of
  them flips. Dragging a window's edge sends nothing until a threshold is
  crossed. The host also drops a report equal to the last one it sent.
- **The start context carries what is known before the first frame.** The
  host starts the app with the platform's pointer and hover (`coarse`/`none`
  on iOS, `fine`/`hover` on macOS), reduced motion from the system setting,
  and on iOS the screen's size class and orientation. A window's real size is
  only known once it is laid out, so the reader's first report corrects
  anything the start context guessed; an app that does not react sees no
  difference.
- **"No reaction" keeps the current props.** The runtime answers an
  environment the app ignores with the current revision and `props: null`
  (§7.1). The Swift host treats an update without props *at the revision
  it is showing* as nothing to render, and keeps showing what it showed. A
  props-less update that moves the revision is a defect and is not papered
  over.
- **Color scheme is the rendered one.** SwiftUI's `colorScheme` is what the
  window actually uses, so the report says `light` or `dark`, never
  `system`.

**Acceptance.** Unit tests pin the generated Swift (the reader around every
runtime-backed root, the `.task(id:)` report, the payload keys and values
matching `mosaic-app-runtime`'s wire names) and the host template (duplicate
reports dropped, a props-less update keeping the current props). The CI
SwiftUI lanes compile the generated app on macOS and for the iOS simulator.
The resize-and-assert gate §7 asks for lands with the first app that reacts:
ENV-last's TaskApp compact layout, launched on a phone.

## 8. Open questions

1. **Should `size-class` thresholds be authorable per component?** A dense
   spreadsheet and a todo list plausibly want different compact points. The
   spec currently says no — thresholds are a host default — because
   per-component thresholds reintroduce pixel reasoning into the authored
   contract. Revisit if a real case appears.
2. **Does `paint` declare environment per render, or per scene?** Per render is
   simpler; per scene would let one artifact rasterize a responsive matrix for
   visual regression, which the Paint gate proposed in
   `task-app-platform-completion-v1.md` would want.
3. **Should `HostNavigationSplit` (§5.4) come first?** If the adaptive
   container lands before variant selection, TaskApp's rail may need no variant
   at all — the control would own the collapse. That would make #13692 a
   consumer of the primitive rather than of ENV3, and would reorder §7.
4. **Does variant selection compose with `--variant` at build time?** A build
   that ships one variant deliberately (UI30 §6 pattern 1) should still be able
   to opt out of carrying all of them. Likely a compile flag; not yet designed.
