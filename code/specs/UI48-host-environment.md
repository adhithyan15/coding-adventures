# UI48 — Host environment: runtime viewport, input modality, and variant selection

**Status:** Specification (decision recorded, not yet implemented)
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
is held, or whether the user asked for reduced motion. Nothing in the stack
carries that information, so nothing can act on it.

UI47 gave the app a way to ask the host to *do* something. This is the inbound
half: a way for the host to tell the app what it *is*.

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

### 5.2 Transport — the standard app ABI

The environment is a struct on `mosaic-app-runtime`, delivered the same way
props are, and exposed across `mosaic-app-capi` so generated native hosts carry
it. A host reports its environment at startup and again whenever it changes.

A host that never reports one gets a documented default —
`regular`/`fine`/`hover`/`landscape`/`light`/`no-preference`. **Every existing
host therefore keeps its current behavior exactly**, which is what makes this
additive rather than a breaking change to seven shipped bindings.

### 5.3 Selection — the kernel picks the variant

The compiled artifact carries every authored variant (UI30's ML5). A
kernel-generated selector maps environment to variant and re-renders when the
environment changes. Authors keep writing `<Component>.touch.mll` exactly as
UI30 specified; what changes is that the choice is no longer frozen at build
time.

Resolution stays UI30's fallback chain: requested variant, then the bare
default. Nothing is required to author every variant.

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

- **ENV1 — vocabulary and runtime type.** The struct, its defaults, and its
  serialization in `mosaic-app-runtime` + `mosaic-app-capi`. No emitter changes;
  no behavior change. Proves the default keeps every existing host identical.
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
3. **Does variant selection compose with `--variant` at build time?** A build
   that ships one variant deliberately (UI30 §6 pattern 1) should still be able
   to opt out of carrying all of them. Likely a compile flag; not yet designed.
