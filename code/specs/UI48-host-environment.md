# UI48 — Host environment: runtime viewport, input modality, and variant selection

**Status:** In progress — ENV1's runtime half implemented in `mosaic-app-runtime` (§7.1); ENV2 and ENV3 on SwiftUI (§7.2), Compose (§7.5), Flutter (§7.9) and Qt (§7.10); ENV4 on SwiftUI (§7.3), Compose (§7.4), Qt (§7.6), XAML (§7.7) and Flutter (§7.8)
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
  an event of that name would be intercepted. `mosmodel-compiler` refuses an
  `emit environmentChanged` as `ReservedName` (UI13 §5), and a test there pins
  its reserved list to `mosaic-app-runtime`'s `ENVIRONMENT_CHANGED`.
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

### 7.4 ENV4 on Compose, designed

The same contract as §7.3, on the Compose shell (`MosaicAppShell.kt`, shared
by desktop and Android since UI89 §3.4). Compose has no layout variants yet
(ENV2/ENV3 on Compose are separate), so this is the report alone.

- **Every runtime-backed app observes.** The strict shell's `MosaicApp`
  wraps its root in `BoxWithConstraints` and reduces what it sees to the six
  §4 values: `sizeClass` from the width against 600 dp and 1024 dp (the
  thresholds SwiftUI's reader uses, so one window size gives one bucket on
  every host), `orientation` from height against width, and `colorScheme`
  from `isSystemInDarkTheme()`. A sample-props shell does not observe.
- **Pointer, hover and reduced motion come from the host.** The runtime host
  already knows its platform, so `MosaicRuntimeHost.initialEnvironment()`
  answers them: `fine`/`hover`/`no-preference` on desktop, `coarse`/`none` on
  Android. Compose Desktop exposes no reduced-motion setting, so that axis
  stays `no-preference` until a platform probe is added. The same values go
  into the start context, as on SwiftUI.
- **Reported on bucket change only.** `LaunchedEffect(report)` is keyed on
  the six values, so it runs once when the window is first measured and again
  only when a bucket flips. The host also drops a report equal to the last
  one it accepted, and remembers a report only once the runtime took it, so a
  refused one is tried again with the next.
- **Through the concrete host.** `MosaicComposeHost` is an interface shared
  with test harnesses and the legacy bridge, and knows nothing about the
  environment; the shell reaches `reportEnvironment` through
  `as? MosaicRuntimeHost`, the same downcast the effect installs use.
- **"No reaction" keeps the current props.** The Kotlin host keeps the props
  it is showing when an update carries none *at the revision it is showing*
  (§7.1), exactly as the Swift host does, and leaves a props-less update that
  moves the revision alone. A runtime refusal (an invalid environment) comes
  back as `{"error": …}` rather than an exception, and the shell applies a
  report's answer only when it carries props, so a refused report never
  replaces what is on screen.

**Acceptance.** Rust tests pin the generated shell (the observer around every
runtime-backed root, the wire names and thresholds, no observer in a sample
shell) and the host template. The Compose conformance harness, run in CI's
Linux lane against the conformance runtime (which ignores the event), checks
that a report keeps the props and revision, an unchanged report is not
resent, a changed one is, and an invalid one is refused, leaves the props and
is not remembered. The resize-and-assert gate lands with the first app that
reacts (ENV-last).

### 7.5 ENV2 and ENV3 on Compose, designed

§7.2 on the Compose shell, reusing what §7.4 already observes.

**ENV2 — one app carries every variant.**

- A variant's root is its own `@Composable`: `<Component><Variant>` in
  PascalCase (`EngramApp.touch.mll` → `fun EngramAppTouch(`), validated the
  way SwiftUI validates `<Component><Variant>View`.
- A variant file carries only what differs: its composable (and its private
  sections, named after it). The component's interface — the `<C>Event`
  sealed class and the `<C>Props` data classes — is emitted once, by the
  default layout's file, and the variant's composable takes and calls those
  same types. Today a variant file repeats all three, so two variants could
  not be compiled together.
- The project shell copies every variant file (`<C>.<variant>.kt`) into
  `src/main/kotlin`, beside the default, so Gradle compiles every layout.

**ENV3 — the selector.**

- The rules are the package's `[[app.layouts]]` (or the conventions),
  computed by the same `effective_layout_rules` as SwiftUI and emitted into
  `MosaicAppShell.kt` as data, in rule order. The generated
  `mosaicLayoutVariant(environment)` returns the first variant whose
  conditions all hold, or null for the default — `select_variant`'s semantics.
- It reads the environment `MosaicApp` already builds for ENV4
  (`environmentReport`), whose keys are the runtime's wire names; each rule
  axis is written under its wire name (`EnvironmentAxis::wire_name`), pinned
  to `mosaic-app-runtime` by a test, so a rule cannot silently test a key the
  report does not carry.
- The root becomes `when (mosaicLayoutVariant(environmentReport)) { "touch"
  -> EngramAppTouch(...) else -> EngramApp(...) }`. The app's state lives in
  the runtime, so swapping roots loses nothing but composition-local state.
- A package without variants gets byte-identical output. A sample-props shell
  with variants observes and selects too (it has no runtime to report to).
- Two variants whose names differ only in letter case or in `-` / `_`
  (`touch` / `Touch`, `task-list` / `task_list` / `tasklist`) are refused when
  the variants are discovered, for every backend: they would name one
  generated view twice, and a case-insensitive filesystem keeps only one of
  their files.

**Acceptance.** Emitter tests pin the variant composable (its name, the
default's event and props types, no redeclaration); builder tests mirror
SwiftUI's four (convention, declared rules, a rule for a missing variant is
refused, no variants → no selector). CI's Linux Compose lane compiles Engram,
the one package with a variant (`EngramApp.touch.mll`), so both roots and the
selector compile together. On desktop the pointer is `fine`, so the touch
layout is compiled but not shown; the resize-and-assert gate still lands
with ENV-last.

### 7.6 ENV4 on Qt, as built

The §7.3 contract on the Qt shell. Qt has no layout variants yet, so this is
the report alone.

- **The host owns the values.** `MosaicHost::environmentReport(width,
  height, dark)` reduces a window to the six §4 values -- `sizeClass` at 600
  and 1024 logical pixels (every host's thresholds), `orientation` from height
  against width (a square window is landscape), `colorScheme` light or dark --
  plus `MosaicHost::initialEnvironment()`: `coarse`/`none` on Android and iOS,
  `fine`/`hover` elsewhere, and `reducedMotion` `no-preference` (Qt has no
  portable setting). The initial values also go into the start context.
- **The shell observes.** Every generated `main.cpp` with a host defines
  `mosaicObserveEnvironment(view, host)` and calls it before the window is
  shown: it reports once, then on `QWindow::widthChanged` /
  `heightChanged` and, on Qt 6.5+, `QStyleHints::colorSchemeChanged` (earlier
  Qts read the palette once). The packaged native-complete shell installs it
  for each host that starts, so a retry's new host is observed and the old
  one's connections go with it.
- **Deduplicated in the host.** `reportEnvironment` sends nothing without a
  runtime or when the report equals the last one the runtime took; it
  remembers a report only once taken, so a refusal is retried with the next
  change (only the identical refused report is not resent).
- **Strict shells get strict answers.** In a native-complete shell
  (`configureRequiredProps`) the answer is checked for required props and
  mapped to QML property names, exactly as `handleRequiredEvent`'s is; a
  missing prop is a refusal, not a half-applied screen.
- **Not in the middle of a settle.** A modal file dialog runs a nested event
  loop inside `settleEffects`, and a resize behind it would dispatch there.
  The shell asks `MosaicHost::isSettling()` first and, if so, retries on one
  restartable 100 ms timer instead.
- **"No reaction" keeps the current props.** `handleEvent` keeps the props it
  is showing when an update carries `props: null` at the revision it is
  showing, as the Swift and Kotlin hosts do -- the runtime's own props, so a
  persistence warning that has since cleared is not kept with them. The shell applies a report's
  answer only when it carries props, and logs a refusal instead of applying
  it (a refusal's props are an empty map, which would blank the screen).

**Acceptance.** The Qt effect driver, linked to the conformance runtime (which
ignores the event), checks the six values and thresholds, that an ignored
report keeps the props and revision, that an unchanged report is not resent,
that an invalid one is refused, leaves the props and is not remembered, and
that a changed one is sent. Emitter tests pin the observer in both
`main.cpp` shapes. As on the other hosts, the resize-and-assert gate lands
with ENV-last.

### 7.7 ENV4 on XAML, as built

The §7.3 contract on the WinUI shell. XAML has no layout variants yet, so
this is the report alone.

- **The host owns the values.** `MosaicRuntimeHost.EnvironmentReport(width,
  height, dark)` reduces a window to the six §4 values from effective pixels
  (WinUI's `ActualWidth`/`ActualHeight`) -- `sizeClass` at 600 and 1024,
  `orientation` from height against width (a square is landscape),
  `colorScheme` light or dark -- plus `MosaicRuntimeHost.InitialEnvironment()`:
  `fine`/`hover`, and `reducedMotion` `no-preference` (WinUI's animation
  setting is not read yet). The initial values also go into the start
  context.
- **The shell observes.** The native-complete `MainWindow` with a component
  root calls `ObserveEnvironment()` once the runtime has started and is
  showing: it reports once, then on the window content's `SizeChanged` and
  `ActualThemeChanged` (the rendered theme). A change is reported from the
  dispatcher queue, never inside the handler (which may fire while an effect
  is being settled), and only one report is queued at a time, so a burst of
  resize ticks costs one. The handlers are wired once, so a retried start
  reports afresh without stacking them; nothing is reported before the first
  layout. A sample shell has no runtime to tell, and a dialog-root window
  shows only the button that opens its dialog, so neither observes.
- **Deduplicated in the host, per runtime.** `ReportEnvironment` sends nothing
  without a runtime, when the report equals the last one the runtime took, or
  when it equals the last one the runtime refused (a drag across a threshold
  would otherwise re-send a refused report on every tick). A refusal does
  not replace the last report taken. A report the runtime took is remembered
  at once, even if showing its answer then fails. A new runtime (after a
  retry) starts with nothing remembered.
- **"No reaction" keeps the current props, and re-applies nothing.** Every
  dispatch and effect completion keeps the props showing when its update
  carries `props: null` at the revision showing (as the Swift, Kotlin and Qt
  hosts do). A report re-applies props to the component only when the
  revision showing is newer than the last one applied: XAML's apply rebuilds
  list view models, which would reset scrolling on every resize. So an apply
  that failed is retried by the next report or event. An answer is applied
  strictly, with the shell's required props, as an event's is.
- **Only a failure reaches the status line.** `ReportEnvironment` answers a
  status only when the report failed (refused, its strict props missing, or
  the runtime closed) or when taking it tripped a settle guard, whose reason
  reaches a caller only through that status; a resize is not something the
  user did, so an accepted report leaves the status describing their last
  action.
- **A backstop inside a settle.** The host sends nothing while a settle is
  running on the same thread and does not remember the report, so the next
  one is sent; the queued shell never reaches it.

**Acceptance.** The XAML conformance harness, run in CI's Windows lane against
the conformance runtime (which ignores the event), checks the six values and
thresholds, that an ignored report re-applies nothing and keeps the props for
the next strict apply, that an invalid report is refused and keeps the props,
and -- through the state file every dispatch rewrites -- that the refused
report is not re-sent, that it did not replace the last report taken, that an
unchanged report is not sent and a changed one is. Rust tests pin the host
template and the shell (the observer after start, wired once, queued, strict,
only failures shown; none in a sample or dialog shell). The TaskApp WinUI
build compiles the observer. As on the other hosts, the resize-and-assert
gate lands with ENV-last.

### 7.8 ENV4 on Flutter, as built

The §7.3 contract on the Flutter shell. Flutter has no layout variants yet,
so this is the report alone.

- **The host owns the values.** `MosaicHost.environmentReport(width, height,
  dark, {reduceMotion})` reduces a window to the six §4 values from logical
  pixels (`MediaQuery`'s size) -- `sizeClass` at 600 and 1024, `orientation`
  from height against width (a square is landscape), `colorScheme` light or
  dark, and `reducedMotion` from the platform's `disableAnimations` -- plus
  `MosaicHost.initialEnvironment()`: `coarse`/`none` on Android and iOS,
  `fine`/`hover` elsewhere, and `reducedMotion` `no-preference`. The binding
  stays free of Flutter imports (the conformance harness runs it on the plain
  Dart VM), so it cannot read the motion setting before the first frame; the
  start context carries the initial values and the shell's first report
  corrects the rest, as on SwiftUI.
- **The shell observes.** The native-complete `main.dart` passes
  `builder: _observeEnvironment` to its `MaterialApp`. The builder runs below
  the app's `MediaQuery` and above every route, and reads only the aspects the
  report needs (`MediaQuery.sizeOf`, `platformBrightnessOf` -- the rendered
  scheme, since `themeMode` is `system` -- and `disableAnimationsOf`), so a
  change to exactly those rebuilds it. It records the report and queues one
  post-frame callback; it never dispatches during build. One report is queued
  at a time and it sends what the last build saw, so a burst of resize frames
  costs one. Nothing is reported until the runtime has started and is
  showing; a retried start reports afresh to its new host. A sample-props
  shell has no runtime to tell, and its host may be a package's own (Venture
  replaces `mosaic_host.dart`), so it does not observe.
- **Deduplicated in the host, per runtime.** `reportEnvironment` sends nothing
  without a runtime, inside a settle (a backstop; not remembered), when the
  report equals the last one the runtime took, or when it equals the last one
  the runtime refused. A refusal does not replace the last report taken; a
  report the runtime took is remembered at once.
- **"No reaction" keeps the current props, and rebuilds nothing.** Every
  dispatch and effect completion keeps the props showing when its update
  carries `props: null` at the revision showing -- the runtime's own props,
  before the persistence warning is folded in, as on Qt. `reportEnvironment`
  answers null when the revision did not move, so the shell calls no
  `setState` for an ignored report; an answer that moved it, or carries a
  tripped settle guard's `error`, is shown exactly as an event's answer is.
- **A failure is logged, never fatal.** The public `reportEnvironment` never
  throws: a refusal (an invalid environment) or a closed runtime comes back as
  `{'error': 'Mosaic environment report failed: ...'}`, which the shell logs
  with `debugPrint`. A resize is not something the user did, so it never
  reaches the startup-failure screen, and a refusal's missing props are never
  applied.

**Acceptance.** The Flutter conformance harness, run in CI's Linux lane
("Round-trip Rust engine through standard Flutter binding") against the
conformance runtime (which ignores the event), checks the six values and
thresholds, that an ignored report has nothing to show and keeps the props and
revision, that an invalid report is refused as an `error` answer and keeps the
props, and -- through the state file every dispatch rewrites -- that the
refused report is not re-sent, that it did not replace the last report taken,
that an unchanged report is not sent and a changed one is. Rust tests pin the
host template and the shell (the builder, the aspect reads, the post-frame
queue, nothing before the host is ready, failures only logged; none in a
sample shell). The same lane runs `flutter analyze`, a launch and the TaskApp
lifecycle widget test on the generated TaskApp, which observes at start. As on
the other hosts, the resize-and-assert gate lands with ENV-last.

### 7.9 ENV2 and ENV3 on Flutter, as built

§7.5 on the Flutter shell, reusing what §7.8 already reduces.

**ENV2 — one app carries every variant.**

- A variant's root is its own Dart widget class, `<Component><Variant>` in
  PascalCase (`EngramApp.touch.mll` → `class EngramAppTouch`), named by
  `mosaic-emit-flutter`'s `variant_widget_name` with the same rule as Compose
  and SwiftUI. A variant whose widget would take a name the default file
  declares (`Card.event-tap.mll` → `CardEventTap`, the `onTap` event) is
  refused, and so is one that would take a public name of the shell's own
  files, which `main.dart` imports beside it (`Mosaic.host.mll` →
  `MosaicHost`; the list, `SHELL_RESERVED_NAMES`, is pinned by tests against
  the generated `main.dart` and the binding templates).
- Known gap, shared with Compose and SwiftUI: a variant widget can still
  collide with *another exported component* of the same package (`Card`'s
  `touch` variant and an exported `CardTouch`). Neither backend checks this
  yet.
- A variant file carries only what differs: its widget and the private
  helpers its own tree uses. A leading `_` makes a Dart name private to its
  file, so those may repeat. The interface — the `<C>Event` sealed class and
  its `<C>Event<Case>` subclasses — is emitted once, by the default layout's
  file. Dart resolves nothing across files without an import, so the variant
  file imports it (`import 'EngramApp.dart';`); the import is always used,
  because the widget's `dispatch` field names `<C>Event`. The variant widget
  takes exactly the default's constructor arguments, since the slots are part
  of the interface too.
- The project shell copies every export's variant files
  (`<C>.<variant>.dart`) into `lib/`, beside the default, so
  `flutter analyze` checks every layout. `main.dart` imports the root's
  selectable variants — only those, because an unused import is an error
  under the generated `analysis_options.yaml`.
- Two variants whose names differ only in letter case or in `-` / `_` are
  already refused when variants are discovered (§7.5); that check is shared
  by every backend, Flutter included.

**ENV3 — the selector.**

- The rules are the package's `[[app.layouts]]` (or the conventions),
  computed by the same `effective_layout_rules` as SwiftUI and Compose, and
  passed to the emitter as `EmitOptions::layout_variants`: one
  `LayoutChoice` per rule, in rule order, each condition keyed by its wire
  name (`EnvironmentAxis::wire_name`). The emitter checks them again before
  writing Dart: a usable variant whose widget is chosen once (keyed on the
  widget, so `touch` and `Touch`, or `task-list` and `task_list`, which name
  one class, are refused together) and is not a shell name, camelCase axis
  names, lowercase values. A rule for a variant with no `.mll` fails the build.
- `main.dart` carries them as data, `mosaicLayoutRules`, a `const` list of
  `(variant, conditions)` records, and a public
  `mosaicLayoutVariant(environment)` that returns the first variant whose
  conditions all hold, or null for the default — `select_variant`'s
  semantics. It is public so a widget test can call it directly.
- Selection runs in a `Builder` placed where the root used to be, below
  `MaterialApp`'s `MediaQuery`: `_mosaicLayoutRoot(context)` reads the same
  aspects §7.8's observer reads (`sizeOf`, `platformBrightnessOf`,
  `disableAnimationsOf`), reduces them with `MosaicHost.environmentReport` —
  so the report and the rules share wire names and thresholds — and switches:
  `case 'touch': return EngramAppTouch(...); default: return EngramApp(...);`.
  A resize across a threshold rebuilds just that `Builder` with the other
  root, on the same frame. The props live in the shell's state (fed by the
  runtime), so swapping roots loses nothing but the old root's own
  widget-local state (a text field's cursor, say).
- A test pins every rule axis's wire name as a key of the Flutter binding's
  `environmentReport`, so a rule can never test a key the report does not
  carry.
- The observer (§7.8) is untouched: it still reports from `MaterialApp`'s
  builder, post-frame. The selector does not wait for it and does not need
  the runtime to have taken the report.
- A sample-props shell with variants selects too, and reports to nobody, as
  on Compose. Its host is the standard binding the builder installs; a
  package that replaces `mosaic_host.dart` and has variants must provide
  `environmentReport` (none does today). The emitter's placeholder host
  gains an `environmentReport` answering an empty environment (the default
  layout) whenever a shell selects from it.
- A package without variants gets byte-identical output: TaskApp and
  RatingControls were emitted before and after, both profiles, with no
  difference. A test also pins that the variant shell, minus the import, the
  `_mosaicLayoutRoot` method and the selector, is byte-for-byte the plain
  shell.

**Acceptance — the resize gate.** Unlike SwiftUI and Compose, Flutter's
widget tests can resize the window (`tester.view.physicalSize`), so the gate
§7 asks for lands here rather than waiting for ENV-last. A fixture package,
`mosaic-emit-flutter/fixtures/layout-variants`, has a default layout and a
`compact` one selected by convention; its widget test mounts the generated
sample shell at 1200 × 800 (default), 400 × 800 (compact), 599 (compact),
600 (default) and back, asserting which root is mounted each time and that
the same props reach it. CI's Linux Flutter lane runs it after `flutter
analyze`. The native-complete shell cannot be mounted in a widget test
without a runtime library, so its selector is covered by Rust tests and by
Engram — the one package with a variant — whose generated native-complete
project the same lane analyzes and builds with both roots, after checking
that `EngramAppTouch`, its import and the `switch` are in `lib/`. On Linux
the pointer is `fine`, so Engram's touch layout is compiled but not shown.

### 7.10 ENV2 and ENV3 on Qt, as built

§7.5 on the Qt shell, reusing what §7.6 already observes.

**ENV2 — one app carries every variant.**

- A variant's root is a QML type of its own, `<Component><Variant>` in
  PascalCase, named by `mosaic-emit-qt`'s `variant_type_name` with the same
  rule as Compose, Flutter and SwiftUI. The file keeps the artifact name every
  backend writes a variant under (`EngramApp.touch.qml`); its type
  (`EngramAppTouch`) is declared in the generated `CMakeLists.txt`, because
  `qt_add_qml_module` otherwise names a file after the text before its first
  dot -- a second `EngramApp`:

  ```cmake
  set_source_files_properties(EngramApp.touch.qml PROPERTIES QT_QML_SOURCE_TYPENAME EngramAppTouch)
  qt_target_qml_sources(EngramApp QML_FILES EngramApp.touch.qml)
  ```

  The project's `qmldir` lists it the same way. A variant whose type would be
  the component's own name, or a name the Qt shell owns
  (`SHELL_RESERVED_NAMES`: `main.cpp`'s `MosaicTableModel` and
  `MosaicLayoutRule`, the binding's `MosaicHost` and `MosaicFileDialogs`,
  pinned by tests against both `main.cpp` shapes and the binding's headers),
  is refused. They are C++ names, which a QML type does not enter today, but
  `MosaicHost::registerTypes()` is where the shell would register its classes
  with QML.
- **Diverges from §7.5: a variant's file declares the interface again.** Its
  slot `property`s, signals, `mosaicEvent` routing and `applyMosaicResponse`
  are the default's, emitted by the same functions from the same `.mil`, and
  the variant's root is exactly the default's root for that tree plus one
  comment line naming it (a test pins this). Compose, Flutter and SwiftUI emit
  the interface once because their files share one namespace, where a second
  `<C>Event` is a redeclaration. QML has neither the problem nor the remedy: a
  file's declarations are members of its own type, so nothing collides, and
  nothing in QML can be imported in their place -- sharing them would mean
  inheriting the default type, whose whole visual tree would be instantiated
  under the variant's. Part of that surface also depends on the layout:
  signal names are allocated against the controls that call them (a `toggle`
  called from a Button becomes `mosaicEmitToggle`), and table models, the
  icon helper and the drag scope exist only when the layout uses them. The one name a variant
  adds to the module is its type, and that is what is checked. The interface
  knowledge the shell needs -- the strict shell's slot names and required
  props -- is emitted once, in `main.cpp`.
- The project shell compiles every export's variants into the QML module, as
  it compiles every export's default. A type the module would register twice
  -- a variant named like another export (`Card` + `touch` beside an exported
  `CardTouch`) or like another variant -- fails the build. (Compose, Flutter
  and SwiftUI do not check a variant against another export's name yet.)
- A native-complete shell mounts every root strictly, so it re-emits the
  root's variants under that policy (`required property var mosaicHost`,
  events through `handleRequiredEvent`) as it re-emits the default. The flat
  artifacts stay permissive.

**ENV3 — the selector.**

- The rules are the package's `[[app.layouts]]` (or the conventions),
  computed by the same `effective_layout_rules` as the other backends, and
  passed to the emitter as `EmitOptions::layout_variants`: one `LayoutChoice`
  per rule, in rule order, conditions keyed by `EnvironmentAxis::wire_name`,
  plus the native table models that variant's root takes. The emitter checks
  them again before writing C++, CMake or `qmldir`: a usable type chosen once
  (keyed on the type, so `touch` and `Touch` are refused together), camelCase
  axis names, lowercase values. A rule for a variant with no `.mll` fails the
  build. A test pins every rule axis's wire name as a key of the Qt binding's
  `environmentReport` (or the `initialEnvironment` it starts from).
- `main.cpp` carries them as data -- `mosaicLayoutRules()`, a list of
  `{variant, source, conditions}` -- with `mosaicLayoutVariant(environment)`
  (the first rule whose conditions all hold, else the default:
  `select_variant`'s semantics) and `mosaicLayoutUrl(environment)`.
- **Selection is in the C++ shell, not in QML.** A `QQuickView`'s root is the
  component itself -- the host sets its properties and calls its
  `applyMosaicResponse` -- so a QML wrapper choosing between roots would have
  to redeclare and forward every slot. Instead the shell swaps the view's
  source. The first root is the one `MosaicHost::environmentReport` of the
  window selects; the §7.6 observer calls `mosaicSwitchLayout` with each report
  it builds, after its settle check and before sending the report, so the
  runtime's answer is applied to the root the window now shows.
- The new root starts with the props the old one showed: in a native-complete
  shell the runtime's, checked and mapped exactly as at startup
  (`propsRequired`); in a sample shell each slot's value is carried across
  from the old root (every layout has the same slots), which covers a runtime's
  props and the shell's own samples alike. The app's state lives in the
  runtime, so nothing is lost but the old root's own QML state. A root that
  cannot get its props or cannot load leaves the window on the layout it was
  showing, and the reason is logged; nothing is thrown from inside the signal.
  Only a layout root is ever swapped for another (never, say, the strict
  shell's startup surface).
- Table models are allocated once, for the largest count any root takes, and
  handed only to the roots that declare them: a QML root refuses an initial
  property it does not declare (checked on Qt 6.4: `setInitialProperties`
  with an unknown name leaves the view in `Error`).
- A sample shell selects too, and reports to nobody. Without the standard
  host (`MOSAIC_HAS_HOST` 0) it has no environment to read and opens the
  default layout.
- A package without variants gets byte-identical output: TaskApp and
  RatingControls were emitted before and after, both profiles, with no
  difference (Engram's changes are exactly the ones above), and a test pins
  the variant project, minus what ENV2/ENV3 add, as the plain one.

**Acceptance — the resize gate.** Qt can resize a window offscreen, so the
gate §7 asks for lands here. A fixture package,
`mosaic-emit-qt/fixtures/layout-variants`, has a default layout (a
`RowLayout`) and a `compact` one (a `ColumnLayout`) selected by convention.
Its harness, `fixtures/layout-variants-test/main.cpp`, compiles the generated
sample `main.cpp` verbatim with its `main` renamed, runs it offscreen, and from
inside its event loop resizes the window to 400, 599, 600, 400 and 1200
logical pixels, asserting each time which root is mounted (by source and by
the layout it built), that the title the first root showed reached it, and
that the host is attached. CI's Linux Qt lane runs it, and before building
Engram -- the one package with a variant -- checks that `EngramApp.touch.qml`
is a strict root and its own type in the module and that `main.cpp` selects
and switches; then it builds the native-complete project with both roots. On
Linux the pointer is `fine`, so Engram's touch layout is compiled but not
shown. Verified locally on Qt 6.4 (the CI lane uses 6.8.3), with the project's
version floor lowered and `RESOURCE_PREFIX /qt/qml` added for 6.4: the
fixture gate passes and fails when the switch or the carried props are
removed, Engram's native-complete project builds, and with a declared
`touch <- size-class = compact` rule the strict shell, running the real
Engram runtime, swaps to the touch root at 400 pixels and back with
`appTitle` carried.

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
