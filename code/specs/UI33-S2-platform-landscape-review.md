# UI33-S2 — Platform landscape review: is the single-IR thesis the right framing?

> **Status.** Strategic review companion to UI33 and UI33-S. Doc-only.
> **Reads honestly about whether UI33's "one .core compiled per
> backend into one dispatcher contract" framing should be retired
> or replaced.** Triggered by two prompts:
>
>   1. The UI33-S survey showed 4 of 7 existing backends already
>      have a major idiom mismatch with the uniform-dispatcher
>      approach.
>   2. Mobile (Android, React Native, MAUI, Compose Multiplatform)
>      should be a first-class concern from day one, not a
>      follow-up — which adds ~6 more emit targets and forces a
>      hard choice about scope.
>
> **The thesis on the table.** Each per-platform emitter should
> become *substantially smarter* — encoding enough knowledge of its
> platform's idioms that it can produce truly native-feeling code
> from a high-level IR. The IR holds the intent; the emitter does
> the heavy reasoning about platform mapping.
>
> **The empirical question this doc forces.** Has anyone ever shipped
> the "single IR → many native UI platforms" approach successfully?
> Because *if not*, we should know that before we build it.
>
> ---

## 0. The empirical record — read this first

Before we design our way around the problem, look at who has tried
this before. The pattern of compromises is striking and consistent.

### 0.1 Every "single source, many native UIs" project compromises one of two axes

The field splits cleanly:

| Camp                          | Strategy                                                                     | What they gave up                                  | Examples                                                             |
|-------------------------------|------------------------------------------------------------------------------|----------------------------------------------------|----------------------------------------------------------------------|
| **Draw our own pixels**       | Render every widget via Skia/Impeller/OpenGL/Metal/etc. on every platform.   | "Feels truly native" (looks consistent, not idiomatic). | Flutter, Compose Multiplatform, Qt, Slint                            |
| **Bind to native widgets**    | Lower into UIKit / Android Views / Win32 / GTK widgets directly.             | "Write once" (abstractions leak; per-platform code creeps in). | React Native, KMM, MAUI, SwiftCrossUI, Skip                         |

Nobody has cracked both. Every project on either side chose its
compromise deliberately and shipped against it.

### 0.2 The cautionary tale Mosaic UI cannot ignore

**Dropbox** famously shipped a shared-C++-for-mobile-business-logic
architecture for ~5 years (mid-2010s). The native iOS and Android
apps both consumed the shared C++ core for sync, file management,
offline state, etc. In 2019 they wrote a public post-mortem about
why they were dismantling it and going back to native-per-platform.
The summary, paraphrased:

- The cost of maintaining the bridge between native UI and shared
  core ended up larger than the cost of rewriting the business logic
  per platform.
- Platform-specific patterns (threading, persistence, lifecycle)
  forced increasing amounts of platform-specific code *inside* the
  shared core.
- Hiring became harder: engineers wanted to work in idiomatic
  platform stacks, not a custom one.

This isn't proof that UI33 is wrong — but the symmetry is uncomfortable.
We are proposing a shared business logic core (`mosaic-core-grid`)
that lowers to 7+ idiomatic targets, with a generated bridge per
target. The Dropbox failure was the same shape.

### 0.3 The two real success stories — both with caveats

- **Flutter** chose draw-our-own-pixels and committed fully. Output
  *is* consistent, *isn't* idiomatic. Cupertino widgets approximate
  UIKit but aren't UIKit. Text selection menus, accessibility tree,
  scroll physics all subtly diverge from platform expectations. The
  bet: most users don't care about this level of fidelity. Mostly
  proven correct in the consumer-app market; mostly false for
  productivity / accessibility-critical apps.
- **React Native** chose bind-to-native-widgets and burned the bridge
  (literally — the old JSON bridge was removed in 0.82; New
  Architecture is default in 0.76+). A `<Text>` IS a real UILabel.
  The bet: keeping native widget primitives means the abstraction
  cost stays bounded. Mostly working — RN is gaining share into 2026.
  Still: every shipped RN app has *some* per-platform code (native
  modules, navigation customizations, etc.).

### 0.4 Spiritual siblings to Mosaic UI

Two recent-ish projects are directly comparable to what UI33 wants
to be:

- **SwiftCrossUI** — A SwiftUI-inspired API that compiles to AppKit,
  UIKit, WinUI, GTK using **native widgets** per platform. Pre-1.0.
  Small but actively developed. *This is the closest existing
  precedent for UI33's ambition.* Its progress (and the gaps where
  it hasn't shipped tier-1 platforms) is the realistic ceiling for
  what a similar effort can achieve.
- **Skip** (open-sourced Jan 2026 by SkipUI Inc) — Transpiles SwiftUI
  source code → Jetpack Compose source code. Single Swift codebase
  ships native iOS + native Android apps with native widgets on
  both. Production-shipped apps exist. *Skip is the most successful
  IR-to-many-native-platforms project to date* — and notice the
  scope is narrow (iOS + Android only, both via known mappings).

The pattern in §0.4: **narrow scope + native widgets = some success.
Wide scope + native widgets = nobody yet.** UI33's seven existing
backends + ~6 mobile additions = unprecedented scope.

---

## 1. The full platform landscape

Updating UI33-S (which covered 7 backends) with the mobile / 2026
state-of-the-art. Each entry has: **state container**, **action
shape**, **adoption status**, **whether Mosaic targets it today**.

### 1.1 Web — modest expansion needed?

| Platform           | State container                     | Action / event shape                   | 2026 adoption       | Mosaic today  |
|--------------------|-------------------------------------|----------------------------------------|---------------------|---------------|
| **React DOM**      | `useReducer`, **Zustand** (now blessed over Redux), TanStack Query for server state | Tagged union; hooks-based dispatch | High (growth)       | ✅ covered    |
| **HTML (static)**  | Plain ES module + EventTarget       | DOM events                             | Universal (stable)  | ✅ covered    |
| **Web Components** | Lit `@property` + CustomEvent       | CustomEvent w/ detail                  | Medium (growth)     | ✅ covered    |
| Vue                | Pinia, Composition API + ref/reactive | Composables                           | High (growth)       | ❌            |
| Svelte             | Stores ($state in v5)                | Built-in reactivity                    | Medium (growth)     | ❌            |
| SolidJS            | createSignal + createStore          | Built-in reactivity                    | Small (growth)      | ❌            |
| Angular            | Signals (v17+), RxJS                | Services + Output emitters             | Medium (stable)     | ❌            |
| Qwik               | useStore, useSignal                 | Resumability primitives                | Small (growth)      | ❌            |

The four extra web frameworks (Vue, Svelte, Solid, Angular) are
each big enough ecosystems that adding them would be ~3-6 months
each. **They are not in current Mosaic scope and should stay out
unless a real demand emerges.**

### 1.2 Apple — UIKit gap, but small

| Platform     | State container                                  | Action / event shape       | 2026 adoption                    | Mosaic today  |
|--------------|--------------------------------------------------|----------------------------|----------------------------------|---------------|
| **SwiftUI**  | `@Observable` (iOS 17+); else `ObservableObject`  | Method calls / Action enum | High (default for new code)      | ✅ covered    |
| UIKit        | No blessed standard; MVVM + Combine `@Published` is closest | Delegate / closure / Combine | Maintenance — new code is SwiftUI | ❌ |
| AppKit       | Same as UIKit (NSResponder hierarchy)             | Same                       | Legacy macOS                     | ❌            |

SwiftUI already covers iOS, iPadOS, macOS, watchOS, tvOS, visionOS
from a single emit target. **UIKit gap is small; not worth a tier-1
emitter.**

### 1.3 Android — major gap

| Platform           | State container                                              | Action / event shape           | 2026 adoption          | Mosaic today  |
|--------------------|--------------------------------------------------------------|--------------------------------|------------------------|---------------|
| **Jetpack Compose**| `ViewModel` + `StateFlow` + `collectAsStateWithLifecycle`; `remember { mutableStateOf }` for UI-only | Lambdas (UDF); side effects in `viewModelScope.launch` | High (default for new code) | ❌ |
| Android Views (XML+Kotlin) | LiveData, ViewModel, Data Binding                    | Click listeners                | Maintenance            | ❌            |

**Android is the biggest gap.** Mosaic UI today cannot target
Android natively at all. If "future mobile apps" is a real
constraint, Jetpack Compose has to land — and *should* land as
tier-1, not as an afterthought.

### 1.4 Microsoft — XAML covered, MAUI/Avalonia gap

| Platform     | State container                              | Action / event shape         | 2026 adoption        | Mosaic today  |
|--------------|----------------------------------------------|------------------------------|----------------------|---------------|
| **WinUI 3 (XAML)** | `CommunityToolkit.Mvvm` `ObservableObject` + `[RelayCommand]` | `ICommand` (RelayCommand) | High (Win desktop) | ✅ covered    |
| WPF          | Same                                          | Same                          | Stable (legacy)      | ✅ effectively (XAML emit works)   |
| .NET MAUI    | Same (XAML or C# markup)                     | Same                          | Stable (Win+iOS+And+Mac) | ❌        |
| Avalonia     | Same                                          | Same                          | Growth (cross-platform XAML, incl. Linux + WebAssembly) | ❌ |

MAUI / Avalonia are largely "XAML emitter with different ceremony"
— the existing XAML emit-target is most of the work. **Adding both
as variants of the existing XAML emitter is medium effort, not a
fresh per-platform investment.**

### 1.5 Cross-platform UI toolkits — the strategic-choice section

| Platform           | State container                              | Action / event shape         | 2026 adoption                                  | Mosaic today  |
|--------------------|----------------------------------------------|------------------------------|------------------------------------------------|---------------|
| **Flutter**        | `ChangeNotifier`, `flutter_bloc` (sealed events+states), Riverpod | Sealed class events / callbacks | High (mobile + desktop + web)             | ✅ covered    |
| **React Native**   | React hooks + Zustand; Expo Router blessed for nav | Tagged union (same as React DOM) | High (mobile, ~Expo)                       | ❌ (RN DOM ≠ React DOM at the emit layer)     |
| **Compose Multiplatform** | `StateFlow` + `ViewModel` (now KMP-common); same as Jetpack Compose | Same as Compose | Production-ready since CMP 1.8 (May 2025); strong growth | ❌ |
| Qt (QML/C++)       | `QObject` + `Q_PROPERTY` + `NOTIFY`           | Signal/slot                   | Stable (desktop); mobile story never won       | ✅ covered (desktop) |
| Tauri              | Rust backend + web frontend; `invoke()` IPC; `tauri::State` | Rust command functions  | Growth (Electron alternative)                  | ❌            |
| Slint              | Property bindings + native rendering         | Signals (similar to QML)      | Small (growth)                                 | ❌            |
| GTK4 (Linux desktop)| GObject + property bindings                  | Signals                       | Stable (Linux desktop)                         | ❌            |
| Kotlin Multiplatform Mobile (KMM, no shared UI) | Per-platform UI (SwiftUI + Compose) over shared business logic in Kotlin | Per-platform native | Strong growth in Android-rooted shops | n/a (KMM is shared logic, not shared UI) |

This row matters most because **the cross-platform toolkits are
strategic alternatives to Mosaic's own approach**. We could plausibly
ship by adopting one of them as the everything-target, instead of
building UI33's IR + emitters from scratch.

### 1.6 The platforms we should *not* try to cover

| Platform           | Why skip                                                              |
|--------------------|------------------------------------------------------------------------|
| TUI (terminal)     | Different paradigm — text-only, no widget tree in the Mosaic sense    |
| Game engines (Unity, Unreal, Godot) | UI is a sub-system inside a larger non-UI runtime    |
| AR/VR (RealityKit, Unity XR) | Different interaction model (spatial, not 2D widget tree)    |
| Voice-first / accessibility-first | Different from "rendered UI" entirely                     |

These are not in scope for this discussion. Mention them only to be
explicit about boundaries.

### 1.7 The honest scope expansion math

If we keep the existing 7 + add what genuinely matters for the
mobile + cross-platform story:

| Adding                | Effort estimate (relative)                    | Coverage gained                                              |
|-----------------------|------------------------------------------------|--------------------------------------------------------------|
| Jetpack Compose       | Heavy (new language, new ecosystem)            | Android native                                               |
| React Native          | Medium (shares React's reducer/hook model)     | iOS + Android via JS                                         |
| Compose Multiplatform | Heavy initially, big payoff                    | iOS + Android + Desktop + Web (Wasm) from one Kotlin target  |
| MAUI / Avalonia       | Light (XAML emitter variants)                  | Cross-Microsoft + Linux/Mac/Mobile via XAML                  |
| Vue / Svelte / Solid / Angular | Medium each                            | Web framework breadth                                        |
| UIKit                 | Medium                                         | Legacy iOS                                                   |

**The "mobile-first-class" decision adds at least Jetpack Compose +
React Native or Compose Multiplatform.** Realistically, three new
heavy emitters at minimum.

---

## 2. Honest assessment of UI33's current shape, given the landscape

UI33 (as merged) commits to:

- A single `.core` DSL that lowers to per-backend reducers + state +
  dispatcher.
- A `.disp` DSL that wires component emits to core actions.
- 7 backends today, with a stated intent to extend.
- A dispatcher pattern that, per UI33-S, is unidiomatic on 3 of 7.

Adding Jetpack Compose + React Native + MAUI + Compose MP to this
approach:

- Multiplies the per-backend emitter count from 7 to ~12.
- Each new emitter is a substantial undertaking (per §1.7).
- The `.core` IR's expressive limits get stretched further: e.g.
  Jetpack Compose's `StateFlow + collectAsStateWithLifecycle` is a
  *very specific* coroutine pattern that's awkward to express
  generically; Compose MP shares it; React Native shares hooks with
  React DOM but the navigation/lifecycle differs.
- The dispatcher pattern's "uniform contract" gets harder to defend
  per added platform.

**This is the moment to honestly ask: is UI33's framing still
right?** Three possible answers, each requires different work:

1. **Yes** — keep going, accept the per-emitter cost, ship the IR-
   to-many-native-targets approach despite the empirical record.
2. **Yes, but narrower** — keep UI33's shape but restrict to a
   pragmatic tier-1 set and *intentionally* delegate the others to
   existing cross-platform stacks.
3. **No** — retire UI33's "single .core for everything" framing.
   Replace with something honest about per-platform-family-divergence.

The recommendation in §5 picks #2 with a strong tilt toward
intelligence-in-the-emitter.

---

## 3. The "smarter backend" thesis, examined

The user's hint: "we might need the backend to become smarter at
what it does." Unpack what this could mean concretely:

### 3.1 What "smarter" can mean

| Axis                   | "Dumb" emitter                                   | "Smart" emitter                                                                                   |
|------------------------|--------------------------------------------------|---------------------------------------------------------------------------------------------------|
| Code generation        | Lowers IR node-by-node to target syntax          | Understands target idioms (composables, modifiers, ICommand) and emits in target's preferred patterns |
| State management       | Generates generic store + dispatch contract      | Generates target-blessed container (`@Observable`, `StateFlow + ViewModel`, `ObservableObject`)   |
| Accessibility          | Emits ARIA on web, ignores elsewhere             | Emits VoiceOver hints / TalkBack labels / Narrator on each target as a default                    |
| Navigation             | Out of scope                                      | Generates `expo-router` routes for RN, `NavigationStack` for SwiftUI, NavHost for Compose, etc.   |
| Hot reload / DevTools  | Out of scope                                      | Emits with the platform's hot-reload story (`Flipper` for RN, `Compose Preview`, `SwiftUI Previews`) preserved |
| Project shell          | UI32 covers ad-hoc shell generation               | Smart emitter knows the platform's build system, manifest format, CI templates, store-listing requirements |
| Testing                | None                                              | Generates platform-blessed test scaffolds (`@Preview`, `XCTest`, `WidgetTester`, `compose-ui-test`) |
| Theming / dark mode    | Static `.msl` mapping                             | Smart emitter integrates target's dynamic-theme APIs (UITraitCollection, DynamicColor, Material You) |

A "smart emitter" doesn't just translate the IR — it knows the
platform deeply enough to produce something a native developer
would actually ship.

### 3.2 What "smart emitter" costs

This is not free. Per platform, "smart emitter" implies:

- Substantial domain expertise encoded in the emitter (months of work
  per platform).
- Tracking the platform's evolution (SwiftUI changes every WWDC;
  Compose changes quarterly; React Native rebuilds its architecture
  every few years).
- Test coverage: each smart emitter needs golden-file tests against
  real platform builds, not just unit tests on the emitter logic.

The math: if a smart emitter is 6× the work of a dumb emitter, and
we want 7 + Jetpack Compose + RN + Compose MP = 10 smart emitters,
that's 60× the work of one dumb emitter. **Realistically, smart
emitters are a tier-1-only investment.**

### 3.3 Smart emitters AS the answer to UI33's idiom mismatch

§3 of UI33-S found that 4 of 7 current backends have major idiom
mismatch with UI33's uniform dispatcher. **A smart emitter would
not have that mismatch** — it would map the IR to the platform's
blessed pattern automatically, regardless of what other backends
do.

This is the strongest argument for the smarter-backend direction:
the idiom mismatch isn't a UI33 spec problem — it's a "current
emitters are too dumb to bridge IR semantics to platform idioms"
problem. Smart emitters fix it structurally.

### 3.4 The risk of smart emitters

The dual of the cost: smart emitters mean the emitter codebase
becomes the largest, most platform-specific, hardest-to-maintain
part of Mosaic. They become **specialty crates** that the wider
contributor pool can't easily touch. Bus factor on each smart
emitter is small.

This is exactly the trade-off the user signaled they want
("intelligence in the framework, not the weights" — but the
intelligence has to live somewhere, and "somewhere" means specialist
maintainers per emitter).

---

## 4. Architectural options — six honest paths

Each option is internally consistent. Pick one (or a hybrid).

### Option A — Continue UI33 as written

Ship the `.core` + `.disp` DSL grammar. Emit per backend in the
current "dumb emitter" shape. Accept the idiom mismatch UI33-S
found. Extend to Jetpack Compose, RN, Compose MP, MAUI by
replicating the existing emitter pattern.

| Pros                                          | Cons                                                                  |
|-----------------------------------------------|-----------------------------------------------------------------------|
| Already in flight (PRs #4627 merged)          | Idiom mismatch on >50% of platforms                                   |
| Single source of truth                         | Empirical record (§0) suggests this approach has a small success ceiling |
| Cheapest per added platform                    | Mobile + cross-platform additions multiply the problem                |
| No new specs                                   | Each new emitter adds dispatcher contract friction                    |

**When to pick:** if velocity matters more than per-platform feel,
and the demos can absorb the awkward output.

### Option B — Smart emitters per platform (the user's "smarter backend" thesis)

Same `.core` + `.disp` IR as UI33. **Every emitter becomes
substantially smarter** — encodes its platform's idioms deeply,
emits idiomatic state containers, accessibility, navigation, theme
integration, project shells. The dispatcher pattern adapts per
backend (reactive method calls on SwiftUI/Qt/XAML/Compose;
event-flow dispatch on React/Flutter/HTML/WebComp/RN).

| Pros                                          | Cons                                                                       |
|-----------------------------------------------|----------------------------------------------------------------------------|
| Output feels native on every platform          | ~6× the per-emitter engineering cost                                       |
| `.core` IR stays as one source of truth        | Specialty maintainer per emitter                                           |
| Idiom mismatch (UI33-S) resolved structurally  | Slow per-platform onboarding; we ship Compose 6 months after announcing it |
| Mobile is tier-1 from day one                  | The IR-to-many-native graveyard (§0) still applies                         |

**When to pick:** if "feels native everywhere" is non-negotiable and
the team can absorb the per-emitter cost. **The user's stated
preference.**

### Option C — Universal UI IR + per-platform-family logic cores

Split the architecture:

- **Mosaic UI (`.mil` / `.mll` / `.msl`) stays universal** — the
  view layer is the one thing that genuinely composes across
  platforms (declarative tree, slot-based composition, kernel
  primitives).
- **Business logic cores fragment by platform family**:
  - Web family (`mosaic-core-web/grid` in TypeScript, used by
    React, Vue, Svelte, Solid, Angular, HTML, Web Components)
  - Apple family (`mosaic-core-apple/grid` in Swift, used by
    SwiftUI on iOS/macOS/etc.)
  - Android/Kotlin family (`mosaic-core-android/grid` in Kotlin,
    used by Jetpack Compose, KMM, Compose MP)
  - Dart family (`mosaic-core-dart/grid`, used by Flutter)
  - .NET family (`mosaic-core-dotnet/grid`, used by XAML, MAUI,
    Avalonia)

Each core is *written natively in its target language family*, so
it's automatically idiomatic. The Mosaic dispatcher then wires
emits in any backend to the appropriate family's core.

| Pros                                          | Cons                                                                      |
|-----------------------------------------------|---------------------------------------------------------------------------|
| Cores are idiomatic by construction           | Cores written 5× instead of once                                          |
| No `.core` DSL needed at all (saves UI33-G-* PRs) | The "single source of truth" thesis is gone                            |
| Family-level reuse is real (one TS core works for 5+ web frameworks) | Cross-family bug-fix coordination becomes a thing       |
| Matches what KMM has actually shipped successfully | Authors of new cores need to know multiple languages                  |

**When to pick:** if you accept that the IR-to-native-everywhere
bet is a graveyard and the honest answer is to write business logic
once per language family, not once for everyone.

### Option D — Tier 1 / Tier 2 platforms

Same as Option B (smart emitters), but explicitly only on a tier-1
subset. Tier-2 platforms get the today-style dumb emitter (or no
core support at all).

Suggested tier split:

- **Tier 1** (smart emitter, full state + override mechanisms):
  React DOM, SwiftUI, Jetpack Compose, Flutter, XAML/WinUI, HTML.
- **Tier 2** (view-emission only, hosts wire their own state):
  Web Components, Qt, MAUI, Avalonia, React Native, Compose MP.
- **Not supported**: Vue, Svelte, Solid, Angular, UIKit.

| Pros                                          | Cons                                                                       |
|-----------------------------------------------|----------------------------------------------------------------------------|
| Honest about where investment goes             | Tier-2 feels second-class to the people who use it                         |
| Tier-1 set is shippable                        | "Why isn't my platform tier 1?" politics                                   |
| Mobile (Compose, RN) gets first-class treatment | Tier-2 platforms can't get core-based business logic — host writes it all |

**When to pick:** as a pragmatic compromise. Avoids over-promising.

### Option E — Adopt an existing cross-platform stack as the everything-target

The honest reframe: **stop emitting native UI per platform, and
ship Mosaic UI as a DSL that emits to one excellent cross-platform
target.**

Realistic candidates:

- **Flutter** — covers iOS, Android, web, desktop. Most mature
  cross-platform widget framework. The draw-pixels compromise (see
  §0.3).
- **Compose Multiplatform** — covers Android, iOS, desktop, web
  (Wasm). Production-ready as of mid-2025. Same draw-pixels
  compromise.
- **React Native + RN-Web** — covers iOS, Android, web. Native
  widgets on mobile, DOM on web. Different runtimes inside but one
  authoring surface.

Mosaic UI's IR (`.mil/.mll/.msl/.core/.disp`) becomes a layer
*above* one of these — emitting only that target. The other
emitters disappear.

| Pros                                          | Cons                                                                      |
|-----------------------------------------------|---------------------------------------------------------------------------|
| The cross-platform target has already solved the IR-to-native problem | Mosaic UI becomes "yet another layer above Flutter / Compose MP" — value-add is real but smaller |
| Massive scope reduction — one tier-1 emitter, not 12 | Lose XAML/WinUI/etc. (the cross-platform targets don't cover Win desktop natively, though Flutter does) |
| Mobile is solved by adoption                  | Inherit the chosen target's compromises (consistent-not-idiomatic)        |
| 90% less engineering work                     | The "intelligence in framework" thesis is gone — Flutter/Compose IS the intelligence |

**When to pick:** if the goal is shipping apps, not building
infrastructure. The fastest path to "I have a Mosaic-UI iOS+Android
app shipping in production."

### Option F — Hybrid: smart emitters for the desktop/web side, adopted target for mobile

- Smart per-platform emitters for **the platforms where "feels
  native" really matters and a smart emitter is achievable**:
  - React DOM (web)
  - SwiftUI (Apple)
  - XAML / WinUI (Windows desktop)
  - HTML (server-rendered)
- **Adopt Compose Multiplatform for mobile** — Jetpack Compose
  emit-target on Android, with the same Kotlin core auto-shared to
  iOS via CMP. Single mobile target instead of "Android emit + iOS
  emit + RN emit + Flutter emit."
- Web Components + Qt + Flutter + RN become **secondary targets**:
  hand-tuned wrappers, no core machinery, deprioritized.

| Pros                                          | Cons                                                                      |
|-----------------------------------------------|---------------------------------------------------------------------------|
| Mobile solved via adoption (smaller blast radius than Option E) | Forces Kotlin + Compose MP as the mobile stack — not everyone's preference |
| Desktop/web stays first-class                  | Two architectural paradigms in one project (smart emit vs. adopted target) |
| Realistic scope                                | Compose MP iOS rendering is still maturing; betting on its trajectory     |

**When to pick:** if you want "feels native everywhere we control,
adopt where the field has already won." This is the most pragmatic
option.

---

## 5. Recommendation

**Option F.** With these specifics:

### 5.1 Reframed scope

| Tier | Platforms | Approach |
|---|---|---|
| **Tier 1 — smart emitter, full Mosaic core support** | React DOM, SwiftUI (iOS+macOS+watchOS+tvOS+visionOS), WinUI 3 / XAML, HTML | Encode platform idioms deeply per emitter. Generated code feels native. State containers use platform-blessed patterns (Zustand-hook / @Observable / ObservableObject+RelayCommand). |
| **Tier 1 — adopted cross-platform stack** | Jetpack Compose (via Compose Multiplatform Android target) | One emitter that compiles Mosaic IR → Kotlin Compose source. Android-first; iOS via CMP comes "for free" as the same emit target. |
| **Tier 2 — view-emission only, host wires logic** | Flutter, Qt, Web Components, MAUI, Avalonia, React Native | Existing-style emitters. No core support; hosts write their own state. Documented gap; could be promoted later. |
| **Out of scope** | Vue, Svelte, Solid, Angular, UIKit, Compose MP on iOS as standalone (covered via tier-1 Compose), Tauri | Stay out unless real demand. |

### 5.2 What changes in UI33

| Section | Change |
|---|---|
| Three-layer architecture (§2) | Stays — but dispatcher is *compile-time-only*, narrows to "wire emits to native-shape core methods per backend" |
| `.core` DSL (§3) | Lives only for Tier-1 platforms. The DSL has to be IDIOM-AWARE — its lowering rules differ per backend (reactive vs event-flow modes). |
| `.disp` DSL (§4) | Lives only for Tier-1 platforms. Tier-2 platforms get a manual host wiring escape hatch. |
| `--emit-project` (§5) | Stays, expands to include Jetpack Compose / Compose MP project shells. |
| Migration plan (§7) | Replaced by §5.3 below. |
| Implementation plan (§8) | Replaced by §5.4 below. |

### 5.3 Migration policy

- **No big-bang migration.** Today's demos keep working.
- Existing Mosaic UI components (`.mil/.mll/.msl`) keep emitting to
  all 7 current backends. View-layer emission is universal —
  *that's not the part we're questioning*.
- The new `.core/.disp` layer ships as **tier-1-only opt-in**.
  Tier-2 platforms ignore it.
- VisiCalc-React migrates first (already planned in UI33).
  VisiCalc-Compose ships next (validates the adopted-stack
  approach). VisiCalc-SwiftUI third (validates smart-emit on
  Apple). The 3 pilots are the architectural validation.

### 5.4 Implementation phasing — replaces UI33 §8

| Phase | PRs | Goal |
|---|---|---|
| **Phase 0** — landscape decisions | This doc + UI33 amendments | Adopt §5.1 scope; mark tier-2 platforms explicitly; retire "uniform dispatcher contract" framing |
| **Phase 1** — `.core` grammar | UI33-G-1..13 | Same as today's UI33 plan |
| **Phase 2** — `.disp` grammar | UI33-D-1..9 + W1 | Same as today's UI33 plan |
| **Phase 3** — React smart emitter | UI33-E-react-1..4 | Reference smart emitter. `useGridCore()` hook + actions object + per-state-slot subscriptions |
| **Phase 4** — `mosaic-core-grid` v0.1.0 + VisiCalc-React pilot | UI33-R-1, UI33-V-react | First validating end-to-end run |
| **Phase 5** — SwiftUI smart emitter | UI33-E-swiftui-1..4 | `@Observable` class + method-call action shape + .onChange-style modifiers |
| **Phase 6** — VisiCalc-SwiftUI pilot | UI33-V-swiftui | Validates smart-emit on a reactive platform |
| **Phase 7** — Compose / Compose MP emitter | UI33-E-compose-1..5 | The mobile-tier-1 emitter. `ViewModel + StateFlow + collectAsStateWithLifecycle`. Lands Android natively; CMP iOS comes from the same emit. |
| **Phase 8** — VisiCalc-Android pilot | UI33-V-android | Validates the adopted-stack approach. Mobile is shippable. |
| **Phase 9** — XAML smart emitter | UI33-E-xaml-1..3 | `ObservableObject` + `[ObservableProperty]` + `[RelayCommand]` via CommunityToolkit.Mvvm |
| **Phase 10** — HTML smart emitter | UI33-E-html-1..3 | Opinionated hydrator + plain ES module + EventTarget. Shared module with WebComp. |
| **Phase 11** — additional Tier-1 cores | UI33-C-* | mosaic-core-form, mosaic-core-list, mosaic-core-tree, mosaic-core-tabs, mosaic-core-router (per UI33 §8 Phase 6) |

**Roughly the same number of PRs as today's UI33 plan, but the
emitter PRs each carry substantially more work** (smart-emit is
~3-6x dumb-emit). Realistic timeline: 3-6 months for Phases 0-8;
another 3 months for Phases 9-11.

### 5.5 What we're explicitly NOT doing

- Not building a Flutter / RN / Qt smart-emitter (they stay tier-2,
  hand-wired hosts only).
- Not building emitters for Vue / Svelte / Solid / Angular.
- Not adopting Flutter / Compose MP / RN as the everything-target
  (i.e. *not* Option E in §4). Tier-1 platforms keep getting
  smart-emit treatment to preserve "feels native" for those.
- Not addressing platform-specific concerns the IR can't sensibly
  express (gesture systems, AR overlays, push notifications, deep
  links). Those stay in host extension points.

---

## 6. What this changes about UI33 (concrete amendments)

If this recommendation lands, UI33 needs:

1. **§2 architecture diagram** — explicitly mark dispatcher as
   compile-time-only. Show the Tier 1 / Tier 2 split.
2. **§3 per-backend table (Grid core emission)** — replace with the
   §3.3 table from UI33-S (the idiomatic per-backend shape table)
   AND add Jetpack Compose row.
3. **§3.4 (no async)** — re-affirm. The host extension-point is
   where async lives, including coroutines on Compose,
   `Task { await }` on SwiftUI, etc.
4. **§4 dispatcher** — narrow to compile-time binding generator.
5. **§5 `--emit-project`** — extend to include Compose / Compose MP
   project shells (`build.gradle.kts`, `composeApp/`, etc.).
6. **§6 reference core** — add VisiCalc-Compose as the second pilot;
   keep VisiCalc-React as the first. Mention VisiCalc-SwiftUI as
   the third.
7. **§7 migration** — replace with §5.3 above.
8. **§8 implementation plan** — replace with §5.4 above.
9. **NEW §X — Tier 1 / Tier 2 / Out of scope policy** — explicit
   list with the criteria for promotion/demotion.

---

## 7. Open questions for the user

Before any of this gets written into a UI33 amendment, the
following need a decision:

1. **Tier-1 set** — confirm React DOM, SwiftUI, Compose (via CMP),
   XAML, HTML? Or different?
2. **Adopted cross-platform stack for mobile** — Compose
   Multiplatform (recommendation above) vs. Flutter vs. React
   Native + RN-Web vs. KMM?
3. **Web framework breadth** — really stay React-only on Tier 1, or
   add Vue/Svelte/Solid? (My recommendation: stay React-only until
   demand emerges.)
4. **MAUI/Avalonia** — keep as Tier-2 view-emission only, or
   promote to Tier-1? They share XAML's emit machinery so cost is
   medium, not heavy.
5. **The "smart emitter" investment** — comfortable with 3-6x the
   per-emitter cost? Comfortable with specialist maintainers per
   emitter?
6. **Tier-2 demotion path for current backends** — Flutter, Qt, Web
   Components, RN are listed as Tier-2 in §5.1. They keep working
   as view-only emitters. Confirm that's OK vs. keeping them all
   tier-1.
7. **VisiCalc as the pilot, three times** — VisiCalc-React,
   VisiCalc-Compose (Android), VisiCalc-SwiftUI (Apple) all need to
   work end-to-end before the architecture is considered validated.
   Three pilots is more work than today's "VisiCalc-React only."
   Comfortable with that gate?
8. **The Skip / SwiftCrossUI question** — given Skip (Swift→Compose)
   exists and is open source, is there a case for *consuming* Skip
   instead of building our own Swift+Compose mapping? Skip is the
   most successful spiritual sibling project; ignoring it is a
   bigger statement than competing with it.

---

## 8. The honest closing

UI33 as written can ship. It will produce working applications.
It will *not* produce applications that feel native on most
backends, because the dispatcher contract is unidiomatic on more
backends than it's idiomatic on.

The mobile expansion makes this 2× worse, not 2× better. We are
about to commit substantial engineering to a pattern that doesn't
yet have a precedent at the scope we're attempting (§0).

The smartest investment is probably:

- Stop multiplying emitters with the same "dumb" approach. Make
  fewer emitters, each substantially smarter.
- Use the adopted-stack trick (Compose Multiplatform) to cover
  mobile + part of desktop with one smart emitter instead of three
  separate ones.
- Be honest about tiers. Don't ship "Mosaic supports 12 backends"
  when 6 of them feel non-native.
- Make sure 3 pilots (one tier-1 reactive, one tier-1 event-flow,
  one adopted-stack mobile) validate the architecture before
  expanding.

This is more conservative than the current UI33. It is also more
likely to ship a thing engineers actually want to use.

---

*End of review. Recommend reading §0 + §5 + §7 even if skipping
everything else.*

---

## 9. Amendment — the component-only exposure constraint

**Added after initial review.** The user clarified a fundamental
constraint that significantly reframes §0 and partly rewrites the
§5 rationale:

> *We will never expose direct-to-pixel layers. We will build
> components on top of them, and those are the only ones that get
> exposed.*

This is one level above where SwiftCrossUI, Skip, React Native,
KMM, and Flutter operate. Those projects expose **widget-level**
abstractions — `SwiftCrossUI.Button` IS the widget the author
types. Mosaic UI exposes **component-level** abstractions —
`mosaic-pkg-toolkit::Button` is a composition of kernel primitives,
which in turn map to either native widgets OR pixel-drawn
approximations depending on the emitter — but the author never
sees that choice.

### 9.1 Why this matters — the empirical record splits in two

§0's cautionary tale was about widget-level cross-platform projects.
At the **component layer**, the precedent is much stronger and
mostly successful:

| Layer  | Cross-platform examples                                                       | Outcome                              |
|--------|-------------------------------------------------------------------------------|--------------------------------------|
| Pixel  | Flutter, Compose MP, Qt, Slint                                                | Visually consistent, not idiomatic    |
| Widget | SwiftCrossUI, Skip, React Native, KMM, MAUI                                   | Native feel, narrow scope, leaks     |
| **Component** | **Material UI, shadcn/ui, Chakra, Bootstrap, Mantine, Radix, MUI-for-X** | **Routine; scaled to millions of apps** |

Component-level cross-platform composition is a *solved problem*.
The library author ships components; consumers compose them;
internal widget choices are an implementation detail. Mosaic UI's
new framing puts us in this row, not the widget row.

### 9.2 Two constraints, not one — they're separable

The exposure clarification reveals there are actually two
constraints in tension, not one:

1. **Author API surface = components only, no pixels and no widgets.**
   Authors compose `<Grid>`, `<FormulaBar>`, `<Button>`. They don't
   write `Skia.drawRect()` or `UIKit.UIButton()` or
   `Compose.Button()`. This is enforced at the IR layer — `.mil`
   only references kernel primitives and other components.

2. **End-user perceived nativeness = whatever the emitter can
   economically achieve.** A Mosaic-iOS app should ideally feel
   indistinguishable from a hand-written SwiftUI app to its end
   user. Where economically possible, the emitter binds to native
   widgets (constraint-2 satisfied fully). Where not, the emitter
   uses a draw-pixels adopted stack (constraint-2 satisfied
   approximately — Material approximates iOS, etc.).

These constraints are *separable*. Constraint 1 is structural
(enforced by the type of thing you can write in `.mil`).
Constraint 2 is qualitative (a per-platform investment decision).

### 9.3 What this changes about §5 — Option F sharpens

The Option-F recommendation in §5 still wins, but the rationale is
now crisper:

| Tier | Honors constraint 1? | Honors constraint 2? | Approach                                                     |
|------|----------------------|----------------------|--------------------------------------------------------------|
| Tier 1 — native-widget emit (React DOM, SwiftUI, WinUI XAML, HTML) | Yes (always) | **Fully** — primitives bind to platform widgets        | Smart emitter encodes platform idioms; native feel preserved |
| Tier 1 — adopted-stack (Compose MP for mobile) | Yes (always) | **Approximately** — Material approximates iOS         | One emitter covers Android + iOS via Skia rendering          |
| Tier 2 — view-only (Flutter, Qt, WC, MAUI, Avalonia, RN) | Yes (always) | Deferred                                                | Existing emitters; no core machinery                         |

The "draw pixels" choice in Compose MP is now an explicit cost we
pay for mobile reach, not a architectural defect. The author never
sees it; the end user sees a Material-approximating-iOS finish
instead of true UIKit. We choose to pay that cost on mobile and not
on desktop/web (where Tier-1 native-widget emit covers it).

### 9.4 What this changes about §0 — most of the anxiety dissolves

§0's empirical cautionary tale (Dropbox, SwiftCrossUI's narrow
scope, the IR-to-many-natives graveyard) was about *widget-level*
cross-platform. With the component-only constraint:

- **Dropbox's failure** was about sharing business logic in C++
  beneath native UI shells. Not what we're doing. Mosaic UI cores
  share *application* logic per language family (per §1.4 / Option
  C-flavoured), not generic business logic, and operate beneath a
  component composition layer that's stable.
- **SwiftCrossUI / Skip** expose SwiftUI-shaped widgets. Mosaic
  exposes components. We can use Skip's *technique* (Swift syntax →
  Compose) without consuming Skip directly, because we don't
  expose Swift syntax to authors.
- **Flutter's "not native" critique** is about end users perceiving
  draw-pixels-doesn't-match-real-platform. This applies to Mosaic
  *only on adopted-stack tier-1 platforms* (Compose MP). On
  native-widget tier-1 platforms (SwiftUI, etc.), we honor
  constraint 2 fully.

§0 still applies to two things:
- Code-sharing-bridges (the Dropbox-style "shared core + native UI"
  bridge maintenance overhead). We hit this only for the
  per-language-family cores. The mitigation is keeping cores small
  and well-bounded.
- Wide-scope investment math. We still face 5-7 smart emitters
  worth of engineering. The component-only constraint doesn't
  reduce that work.

### 9.5 What this changes about §7 — three of the eight questions resolve

| Q  | Question                                                              | Resolved by §9?                                                                                              |
|----|-----------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------|
| Q1 | Tier-1 set                                                             | No change.                                                                                                   |
| Q2 | Mobile adopted stack                                                   | **Compose MP becomes more defensible** — its draw-pixels compromise is invisible to our authors.            |
| Q3 | Web framework breadth                                                  | No change.                                                                                                   |
| Q4 | MAUI / Avalonia tier                                                   | No change.                                                                                                   |
| Q5 | Smart-emit investment                                                  | **Same engineering cost, but the OUTPUT QUALITY constraint reduces** — smart-emit's job is now "make primitives feel native at the component-composition level," not "expose every widget feature idiomatically." Mildly easier. |
| Q6 | Tier-2 demotion                                                        | No change.                                                                                                   |
| Q7 | Three pilots                                                           | No change.                                                                                                   |
| Q8 | Skip / SwiftCrossUI                                                    | **Resolved — we do NOT consume Skip.** Skip exposes SwiftUI syntax; Mosaic exposes its own component DSL. We can study Skip's Swift→Compose mapping algorithm for our Android emitter inspiration, but we don't depend on it. SwiftCrossUI similarly informs but doesn't get adopted. |

### 9.6 The component catalogue becomes the visible deliverable

Under this framing, the deliverable users *interact with* is the
component catalogue:

- `mosaic-pkg-toolkit` — Button, Input, Checkbox, Radio, Select,
  Switch, Slider, Tabs, Accordion, Dialog, Toast, Tooltip,
  Popover, Menu, Card, Avatar, Badge, Progress, Spinner
- `mosaic-pkg-grid` — Grid, Cell, Column, Row, HeaderRow, etc.
- `mosaic-pkg-form` — Form, FormField, FieldError, FieldGroup,
  Validation
- `mosaic-pkg-list` — List, ListItem, SortableList, FilterableList
- `mosaic-pkg-tree` — Tree, TreeNode, ExpandableTreeNode
- `mosaic-pkg-router` — Route, Link, Outlet, useRoute equivalent
- `mosaic-pkg-data-table` — DataTable, ColumnDef, SortIndicator
- `mosaic-pkg-charts` — bar / line / area / pie / scatter as
  components (NOT pixel APIs — composed of primitives that map to
  SVG / native charting widget per platform)

Each component is authored once in `.mil/.mll/.msl` (the universal
view IR) plus its `.core` for business logic. The catalogue's
*breadth* is now the real engineering frontier — not the per-
backend emitter count.

This is also closer to how mature component libraries work in any
single ecosystem (shadcn, MUI, Mantine ship hundreds of components
each). The cross-platform multiplier is what makes ours hard, but
the component-only framing keeps the problem bounded.

### 9.7 What still doesn't go away

- **Per-platform widget mapping is still a real engineering
  surface.** Each kernel primitive (HostInput, HostButton, etc.)
  needs a per-platform mapping to native widget or fallback. That
  table is the bedrock; getting it right takes work.
- **Component breadth is now the bottleneck.** Authors expect a
  modern component library. Building 50+ components × N backends ×
  state machinery is a multi-quarter effort even with smart
  emitters.
- **End-user accessibility / theming / RTL / dark-mode per
  platform** still needs investment per platform. Not free with
  the component framing.
- **The dispatcher / core machinery from UI33** is still needed —
  the component-only constraint is orthogonal to the state-and-
  logic question. UI33 stays largely intact; just §6 (reference
  core) might want a worked component-composition example, not
  just the Grid core in isolation.

### 9.8 Updated recommendation

Same as §5 (Option F) but with sharper rationale:

> Mosaic UI is a **component framework with cross-platform native
> emission**. Authors compose components in the universal `.mil /
> .mll / .msl / .core / .disp` DSL set. The emitter ecosystem
> splits into tier-1 (native-widget smart emit) and tier-1-adopted
> (Compose MP for mobile, paying a draw-pixels cost that authors
> don't see) and tier-2 (view-only). Component-only exposure means
> the empirical record of widget-level cross-platform failures
> (Dropbox, SwiftCrossUI's narrow scope) doesn't apply to us at the
> same scale.

### 9.9 One new question Q9 added to §7

9. **The component catalogue scope** — what's the v0.1.0 catalogue?
   Just Grid + FormulaBar (the existing VisiCalc set)? Or do we
   front-load Button, Input, Form, List, Dialog, etc. to validate
   that the framework works at component-library scale, not just on
   a single component?

---

*End of amendment §9.*

---

## 10. Amendment — native UI per platform, NOT cross-platform visual consistency

**Added after §9.** User clarified the perceived-nativeness goal:

> *I am not trying to build pixel-perfect UI in every platform. I
> am trying to native UI for each platform.*

Translation: each platform should render in **its own** native
widget style. Cross-platform visual consistency is **not** a goal.

- iOS should look like iOS (UIKit / SwiftUI widget style).
- Android should look like Android (Material / Compose widget style).
- Windows should look like Windows (Fluent / WinUI widget style).
- Web should look like the web (browser-native form controls,
  modern flat styling, however the author skins it).

This is what we want; *visual sameness across platforms is not even
a secondary aim*.

### 10.1 Why this is a bigger shift than it sounds

§9's component-only constraint resolved *one* tension (the IR layer
shouldn't expose pixels or widgets). This new constraint resolves a
different one: **end-user perceived nativeness IS a first-class
requirement**, not the "approximately satisfied when convenient"
framing §9.3 used.

§9.3 had a row that read:

> Tier 1 — adopted-stack (Compose MP for mobile) — Honors
> constraint 2 *approximately* — Material approximates iOS

That "approximately" was hand-waved over an actual user-visible
compromise. Compose MP on iOS draws Material/Compose widgets via
Skia. They DO NOT look like UIKit. End users on iOS see "this
isn't a real iOS app." Under the new constraint, **that's not
acceptable**.

The same critique applies to Flutter (renders its own widgets on
all platforms — Cupertino approximates iOS, Material approximates
Android, neither is native), Qt mobile (Qt Quick widgets on iOS
look like Qt, not iOS), and any other draw-pixels cross-platform
toolkit.

### 10.2 The recommendation matrix updates

§9.3's matrix had one tier-1 native-widget row plus one tier-1
adopted-stack row. **Drop the adopted-stack row.** Every tier-1
platform gets a native-widget emitter:

| Tier | Platforms (UPDATED) | Approach |
|------|---------------------|----------|
| **Tier 1 — native-widget emit** | React DOM, SwiftUI (iOS+macOS+watchOS+tvOS+visionOS), Jetpack Compose (Android), WinUI XAML (Windows), HTML | Smart emitter encodes platform idioms deeply; output uses **the platform's own native widgets**; end-user experience is genuinely native |
| **Tier 2 — view-only, no smart emit** | Flutter, Qt, Web Components, MAUI, Avalonia, React Native, Compose MP | Existing-style emitters; useful for hosts who want them but they don't honor constraint 2 (native per-platform feel) |
| **Out of scope** | Vue, Svelte, Solid, Angular, UIKit (covered by SwiftUI), Tauri | Stay out unless real demand |

The **Compose MP "free mobile coverage" shortcut is removed**. Mobile
requires two separate tier-1 native-widget emitters: SwiftUI for
Apple platforms (already in scope) and Jetpack Compose for Android
(net-new). Both are first-class.

### 10.3 What this changes about §7's open questions

| Q  | Question                                                              | Re-resolved by §10                                                                                          |
|----|-----------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------|
| Q1 | Tier-1 set                                                             | **Updated.** React DOM + SwiftUI + Jetpack Compose + WinUI XAML + HTML. (Five tier-1 native-widget emitters, no adopted stack.) |
| Q2 | Mobile adopted stack                                                   | **RESOLVED — no adopted stack.** Two separate native-widget emitters for mobile (SwiftUI for iOS, Jetpack Compose for Android). |
| Q3 | Web framework breadth                                                  | No change.                                                                                                   |
| Q4 | MAUI / Avalonia tier                                                   | **Tier-2.** They don't produce native-per-platform output any better than the XAML emitter already does on Windows. MAUI on Mac/iOS/Android is its own non-native style. |
| Q5 | Smart-emit investment                                                  | **Cost goes UP.** Five tier-1 emitters instead of "four tier-1 + one adopted stack." Jetpack Compose emit is net-new heavy work. |
| Q6 | Tier-2 demotion                                                        | No change — Flutter, Qt, WC, RN, MAUI, Avalonia, Compose MP all tier-2 now (Compose MP added to the demoted list). |
| Q7 | Three pilots                                                           | **Changes to four pilots.** VisiCalc-React, VisiCalc-SwiftUI, VisiCalc-Compose-Android, plus one tier-2 sanity-check (VisiCalc-Flutter — to confirm view-only-emit still works). |
| Q8 | Skip / SwiftCrossUI                                                    | **Re-resolved more interesting.** We don't *consume* Skip, but Skip's Swift→Compose widget-mapping logic is now directly relevant: it's exactly the kind of platform-equivalent translation our Jetpack Compose emitter has to internalize. **Study Skip's open-source mappings as engineering reference for our Compose emitter.** |
| Q9 | Component catalogue scope                                              | No change (§9.9).                                                                                            |

### 10.4 The Skip reconsideration (Q8 nuance)

Skip is open-sourced, MIT-licensed, and ships production apps.
What Skip does internally is: take SwiftUI source, AST-parse it,
and emit Kotlin Jetpack Compose source. The hard work — what
SwiftUI button maps to what Compose composable, how state flows,
how navigation translates — is encoded in Skip's transpiler.

We don't author in SwiftUI, so we can't consume Skip end-to-end.
But our `mosaic-emit-jetpack-compose` will have to solve the
**same widget-mapping problem** (Mosaic kernel primitive →
Jetpack Compose composable). Skip's mappings are a free reference
implementation of platform-equivalent translation that we can study,
verify, and steal patterns from.

This makes Skip MORE relevant under §10, not less. We're not
consumers, we're peers studying open-source prior art for a piece
of work we have to do ourselves.

### 10.5 Engineering math, re-honest

§5's Phase 7 was "Compose / Compose MP emitter — one emitter,
mobile + iOS covered." That collapse-of-mobile-into-one-target is
gone. Updated phasing:

| Phase | PRs                    | What it ships                                                                                |
|-------|------------------------|----------------------------------------------------------------------------------------------|
| 7a    | UI33-E-compose-1..5    | **Jetpack Compose** smart emitter (Android only). State container: ViewModel + StateFlow. Project shell: Android Studio-friendly Gradle. |
| 7b    | UI33-V-android         | VisiCalc-Android pilot                                                                       |
| (removed) | ~~UI33-V-ios-via-CMP~~ | Compose MP iOS would have come "for free"; no longer applicable                          |

The iOS coverage continues to come from the SwiftUI emit-target
(Phase 5, already planned). Apple gets first-class native; Android
gets first-class native; they don't share an emitter.

**Cost impact:** Jetpack Compose emitter is heavy. Probably the
single largest emitter investment in the plan (new language, new
toolchain, new state container idiom, new project layout). Maybe
2-3 months of focused work for the emitter alone, plus additional
time for the reference Grid core's Compose mapping.

### 10.6 What we explicitly accept under §10

- Mobile coverage doubles in cost (two emitters, not one).
- "Write Mosaic-UI app, build for iOS AND Android" still works, but
  the per-platform emitters do the work, not an adopted cross-
  platform stack.
- We lose Flutter / Compose MP / Qt as serious tier-1 candidates.
  They become tier-2 escape hatches for hosts who specifically want
  them.
- Linux desktop becomes a real question: there's no native-widget
  tier-1 emitter for Linux unless we add GTK4. (Deferred — Linux
  desktop is small market for new apps; revisit if demand emerges.)
- The "intelligence in the framework" thesis sharpens. Each smart
  emitter has to be deeply native-platform-fluent because the
  output IS the user-visible native UI. No cross-platform-uniform
  layer hides the per-platform fluency.

### 10.7 What we explicitly do NOT accept under §10

- **Authors compose components, not platform widgets.** §9's
  constraint is preserved exactly. Authors write
  `mosaic-pkg-toolkit::Button`; on iOS that emits a SwiftUI
  `Button(...)`, on Android it emits a Compose `Button(...)`, on
  Windows a `<Button>` XAML element. Author writes once. End user
  sees genuinely native per platform.
- **No theme abstraction trying to make Material look like iOS or
  vice versa.** That's the wrong direction. Each platform looks
  like itself.
- **No "Mosaic look-and-feel."** There is no Mosaic visual identity
  that overrides platform style. Mosaic is INVISIBLE to end users —
  they just see iOS-native or Android-native or web-native UI,
  authored once at the component level.

### 10.8 Updated final recommendation

> Mosaic UI is a **component framework that emits genuinely native
> UI for each target platform**. Authors compose components in the
> universal `.mil / .mll / .msl / .core / .disp` DSL set. The
> emitter ecosystem is FIVE tier-1 native-widget smart emitters
> (React DOM, SwiftUI, Jetpack Compose, WinUI XAML, HTML); a tier-2
> set of view-only emitters for ecosystems that want them; and
> nothing else in scope. There is no adopted cross-platform stack.
> Skip and SwiftCrossUI are studied for their widget-mapping
> patterns but not consumed. The end-user experience on every tier-1
> platform is indistinguishable from a hand-written native app on
> that platform.

---

*End of amendment §10.*

---

## 11. Amendment — cross-platform-consistency is an OPT-IN target choice, not a tier-2 demotion

**Added after §10.** User insight:

> *If someone wanted a pixel-perfect UI in all platforms, they can
> emit to Flutter and then compile it down to all the platforms?*

Yes — exactly. And this insight resolves something §10 didn't
acknowledge cleanly: **there are two legitimate philosophies
authors might want, and the framework can serve both** by treating
the emit target itself as a user choice.

### 11.1 The two legitimate philosophies

| Philosophy                                       | Want                                                              | Best-fit emit target                                                |
|--------------------------------------------------|-------------------------------------------------------------------|---------------------------------------------------------------------|
| **Native UI per platform** (§10's default)       | iOS to look like iOS, Android like Android, web like the web      | Tier-1A native-widget emit (SwiftUI, Jetpack Compose, React DOM, WinUI XAML, HTML) |
| **Pixel-perfect cross-platform consistency**     | One UI authored once, looks identical everywhere it ships          | Tier-1B adopted-stack emit (Flutter primary; Compose MP, Qt as alternatives) |

These aren't right vs. wrong. They're different choices for
different apps:

- **A B2C iOS-first app** wants native UI per platform — the
  iOS-native feel is part of the brand and the App Store reviewer
  will notice if it's not.
- **An enterprise productivity tool** wants pixel-perfect
  consistency — the company has trained users on one UI and the
  rendering deviations between platforms are a support burden.
- **A consumer game / drawing app** wants pixel-perfect consistency
  — the brand IS the look, and platform-native widgets would clash.
- **A native iOS extension built on Mosaic** wants native UI
  per platform — extension UI needs to feel like the host app.

Mosaic UI should support both. The author picks per-project (or per
deploy target).

### 11.2 The tier model expands

§10's tier-2 row "Flutter, Qt, WC, MAUI, Avalonia, RN, Compose MP"
collapsed too much. The right split:

| Tier | Sub-tier | Platforms | Approach |
|------|----------|-----------|----------|
| **Tier 1A** | Native-widget smart emit | React DOM, SwiftUI, Jetpack Compose, WinUI XAML, HTML | "Native UI per platform" philosophy; the platform's blessed widgets are the output |
| **Tier 1B** | Cross-platform-consistency smart emit | **Flutter** (primary), Compose MP, Qt | "Pixel-perfect consistency" philosophy; one binary, one look, all platforms |
| **Tier 2** | View-only / niche | Web Components, MAUI, Avalonia, React Native, GTK4 | Useful for specific hosts; no smart core emit unless demand emerges |
| **Out of scope** | — | Vue, Svelte, Solid, Angular, UIKit (covered by SwiftUI), Tauri (covered by HTML+web emit), Slint | Stay out unless real demand |

Flutter is the most defensible Tier-1B primary choice:

- Most mature draw-pixels cross-platform stack.
- Covers iOS, Android, web, desktop (Win/Mac/Linux) from one
  codebase.
- Idiomatic state container (`flutter_bloc`) maps cleanly to
  `.core` reducer semantics.
- Largest ecosystem of Tier-1B-philosophy adopters.

Compose MP and Qt are alternatives with their own trade-offs:
Compose MP for Kotlin shops; Qt for desktop-first / C++ shops.

### 11.3 What this changes about the rest of the doc

| Section            | Change                                                                               |
|--------------------|--------------------------------------------------------------------------------------|
| §0 (empirical record) | The "draw-pixels-doesn't-feel-native" critique applies only to **Tier 1A** mismatches. Tier 1B users *want* draw-pixels because consistency is the goal. The graveyard is half a graveyard. |
| §5 (recommendation)| Update tier classification to include Tier 1B as first-class. The investment per emitter increases but is justified by serving both philosophies. |
| §9.3 (matrix)      | Replace adopted-stack-as-mobile-shortcut framing with adopted-stack-as-user-choice framing. |
| §10.2 (matrix)     | Promote Flutter / Compose MP / Qt from "Tier 2" to "Tier 1B". They're not failures of Tier 1A — they're the answer for a different question. |
| §10.6 (accepted costs) | Mobile cost doubles only for the Tier 1A path. Tier 1B users get mobile-and-desktop-and-web from one emit target (Flutter). |

### 11.4 The new mental model for emit targets

A Mosaic UI project's `mosaic.toml` (or equivalent) declares emit
targets per deploy slot:

```toml
[targets.production]
# An iOS-first consumer app, native feel preferred everywhere
ios     = "swiftui"          # Tier 1A
android = "jetpack-compose"  # Tier 1A
web     = "react-dom"        # Tier 1A
windows = "winui-xaml"       # Tier 1A

[targets.kiosk]
# A kiosk app shipped to industrial Android+Linux tablets, must look identical
all-platforms = "flutter"   # Tier 1B — one binary, one look

[targets.preview]
# A web-only preview build for product review
web = "html"                 # Tier 1A (server-rendered)
```

Authors pick per-deploy-slot. The framework wires whichever emit
target the slot specifies. The component source is the same across
all slots — only the emitter differs.

### 11.5 Implementation phase impact

§10.5 added Phase 7a (Jetpack Compose). §11 adds:

| Phase | PRs                    | What it ships                                                                            |
|-------|------------------------|------------------------------------------------------------------------------------------|
| 8     | UI33-E-flutter-1..5    | **Flutter smart emit** (Tier 1B primary). State container: `flutter_bloc` sealed events + states. Project shell: pubspec.yaml + lib/main.dart. |
| 8b    | UI33-V-flutter         | VisiCalc-Flutter as the Tier 1B pilot (proves cross-platform-consistency philosophy ships end-to-end) |
| 9     | UI33-E-compose-mp-1..3 | **Compose MP smart emit** as Tier 1B alternative (smaller scope than Flutter — JVM-friendly shops) |
| 10    | UI33-E-qt-smart-1..3   | **Promote existing Qt emitter to smart-emit** (state container: QObject Q_PROPERTY+Q_INVOKABLE; project shell: CMakeLists.txt + main.cpp) |

Pilot story (§5.4 + §10.5 + §11) becomes:

- VisiCalc-React (Tier 1A native web)
- VisiCalc-SwiftUI (Tier 1A native Apple)
- VisiCalc-Compose-Android (Tier 1A native Android)
- VisiCalc-Flutter (Tier 1B cross-platform consistency) ← promoted from sanity-check to philosophy validator

Four pilots — each validates one architectural axis. If all four
ship, the architecture is proven across the two philosophies and
across the smart-emit cost target.

### 11.6 The hidden bonus — Tier 1B amortizes mobile

Authors who want Tier 1B get *iOS + Android + web + desktop* from a
single Flutter emit target. **For a small team that doesn't have the
appetite for two native-mobile emitters (SwiftUI + Jetpack Compose
separately), Tier 1B IS their mobile story.**

This means:

- Solo developers / small teams can ship to iOS+Android+web by
  picking Tier 1B Flutter for their whole project.
- Established companies with platform-native standards pick Tier 1A
  per platform.
- Mosaic UI's component source is the same. The choice is at the
  emit-target level, not the IR level.

That's the strongest argument for why Mosaic-UI-with-both-tiers is
more valuable than either tier alone.

### 11.7 §7 open question Q2 re-re-resolves

| Q | Previous resolution                                                                          | Updated by §11                                                                                                  |
|---|-----------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------|
| Q2 | §10: "RESOLVED — no adopted stack. Two separate native-widget emitters for mobile."           | **Updated.** Both paths exist. Tier 1A users get separate SwiftUI + Jetpack Compose emit. Tier 1B users get Flutter (or Compose MP) emit covering iOS + Android + more. |

### 11.8 Updated final recommendation (replaces §10.8)

> Mosaic UI is a **component framework with two-philosophy emit
> target ecosystem**: Tier 1A native-widget emit (React DOM,
> SwiftUI, Jetpack Compose, WinUI XAML, HTML) for hosts who want
> genuinely native UI per platform; Tier 1B cross-platform-
> consistency emit (Flutter primary; Compose MP, Qt as alternatives)
> for hosts who want one binary that looks identical everywhere.
> Authors pick per-project or per-deploy-slot. Component source is
> identical across both tiers — only the emitter differs. Tier 2
> exists for niche hosts; the rest is out of scope.

---

*End of amendment §11.*
