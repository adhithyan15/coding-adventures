# UI29-3 — Next kernel-promotion batch: form-control survey

> **Status.** Survey + spec draft. No implementation in this PR.
>
> **Parent.** UI29 — Primitive Kernel + Userland Component Packages
> (`code/specs/UI29-primitive-kernel.md`).
>
> **Predecessors.** UI29-1 (`HostDialog`, kernel primitive #16);
> UI29-2 (`HostCheckbox` + `HostRadio`, primitives #17 + #18).
>
> **Scope.** Surveys three follow-up candidates against the UI29 §2.2
> inclusion criteria, picks a recommended next batch, and pins the
> slot/emit surface for each accepted primitive. Per-backend lowering
> design is sketched but full per-backend specs (one per primitive,
> following the UI29-1 / UI29-2 template) are reserved for the
> implementation PRs.

---

## 1. The candidates

This survey looks at three controls. Each is currently *missing* from
the kernel; each has a userland equivalent already (or a planned
slot) in `mosaic-pkg-toolkit`. The question for every one: same as
UI29-1 §1 and UI29-2 §1 — does the userland composition lose
something that only the native widget provides, and *can* that thing
be composed?

| Candidate     | Current toolkit shape                | Native equivalent on every backend?              |
|---|---|---|
| `HostSelect`  | (not in toolkit yet)                 | `<select>` / `Picker` / `ComboBox` / `<select>`  |
| `HostSlider`  | (not in toolkit yet)                 | `<input type="range">` / `Slider` / `Slider`     |
| `HostProgress`| `Spinner` (animation, not progress)  | `<progress>` / `ProgressView` / `ProgressBar`    |

Three further candidates were considered and rejected for this batch:

- **`HostMenu`** — interactions vary too much across platforms
  (menubars vs. context menus vs. action sheets) for a single
  cross-backend primitive. Better as a userland composition over
  `HostButton` + `HostDialog` for now.
- **`HostTabs`** — same problem; tab strips have widely divergent
  visuals and a11y patterns per platform. Userland composition.
- **`HostTextarea`** — already covered by `HostInput`'s pre-existing
  `multiline:` keyword (legacy `Input` primitive). Promoting a
  separate `HostTextarea` would be redundant.

## 2. Inclusion criteria recap

UI29 §2.2 sets the bar for kernel promotion:

1. **Every host platform has a native equivalent.** The widget must
   exist as a first-class control on the four supported backends
   (DOM, SwiftUI, Qt, XAML).
2. **No reasonable composition exists.** The primitive must own
   accessibility role, focus ring, keyboard semantics, or platform-
   specific behaviour that composition from existing kernel pieces
   provably loses.

UI29-1 and UI29-2 both passed these criteria. The three candidates
below are evaluated against them.

## 3. Per-candidate evaluation

### 3.1 `HostSelect` — dropdown / picker

**Native widgets:**

- DOM: `<select>` with `<option>` children.
- SwiftUI: `Picker { ForEach { Text(...) } }` with
  `.pickerStyle(.menu)` (or `.wheel`, `.segmented`).
- Qt: `ComboBox { model: ... }` from QtQuick.Controls 2.
- WPF / WinUI: `<ComboBox><ComboBoxItem>...</ComboBoxItem></ComboBox>`.

**What composition loses:**

- A11y role (`role="listbox"` / `AccessibilityRole.menu` /
  `AutomationPeer.ComboBox`) and the platform's screen-reader
  integration (announces "1 of 5" etc.).
- Native dropdown UX: on iOS this is a wheel picker; on macOS a
  pop-up menu; on Windows a drop-down with virtualised scroll for
  long lists; on web a true `<select>` that uses the OS picker on
  mobile. A composed `Box`+`Column`+`HostButton` cannot reach any
  of these.
- Keyboard semantics: type-to-search, Enter/Esc, arrow-key
  navigation through options. Custom-built dropdowns famously get
  this wrong.

**Verdict:** ✅ Accept. Strongest case of the three.

**Proposed slot/emit surface:**

| moslayout prop  | Kind     | Required | Meaning                                          |
|---|---|---|---|
| `options`       | slot ref | yes      | list of `{value: text, label: text}` records     |
| `selected`      | slot ref | yes      | currently-selected option's `value` (text)       |
| `disabled`      | slot/kw  | no       | greys out the picker                             |
| `placeholder`   | slot/str | no       | text shown when `selected` is empty              |
| `onChange`      | emit     | no       | fires when selection changes; payload `value: text` |

A simpler "list of strings" variant (`options: list of text`) is
appealing but loses the value↔label distinction every real-world
picker needs. Sticking with the record shape from day one.

### 3.2 `HostSlider` — numeric range input

**Native widgets:**

- DOM: `<input type="range" min max step value>`.
- SwiftUI: `Slider(value: ..., in: min...max, step: ...)`.
- Qt: `Slider { from; to; stepSize; value; orientation }` from
  QtQuick.Controls 2.
- WPF / WinUI: `<Slider Minimum Maximum StepFrequency Value/>`.

**What composition loses:**

- A11y role (`role="slider"` / `AccessibilityRole.adjustable`)
  and the screen-reader value/range announcement.
- Drag UX: pointer drag, touch drag, keyboard arrow ± step,
  Home/End to bounds, Page Up/Down. None of these can be
  reasonably re-implemented in pure layout primitives.
- Visual: thumb + track rendering is platform-specific (and
  themed on iOS / macOS / Windows). A composed slider looks
  hand-rolled and wrong.

**Verdict:** ✅ Accept. Clear case.

**Proposed slot/emit surface:**

| moslayout prop  | Kind        | Required | Meaning                                       |
|---|---|---|---|
| `value`         | slot ref    | yes      | current numeric value                         |
| `min`           | num literal | yes      | range minimum (compile-time choice)           |
| `max`           | num literal | yes      | range maximum (compile-time choice)           |
| `step`          | num literal | no       | step increment (default 1; 0 = continuous)    |
| `disabled`      | slot/kw     | no       |                                                |
| `onChange`      | emit        | no       | fires per drag tick; payload `value: number`  |
| `onCommit`      | emit        | no       | fires on pointer release / blur; payload same |

`onChange` fires continuously (every drag tick); `onCommit` fires
once when the user releases. Some hosts only care about the final
value — exposing both lets them choose. Mirrors the
`HostInput.onChange` / `onCommit` split that's already in the
kernel.

### 3.3 `HostProgress` — progress bar

**Native widgets:**

- DOM: `<progress value max>`.
- SwiftUI: `ProgressView(value:total:)` (determinate) /
  `ProgressView()` (indeterminate spinner).
- Qt: `ProgressBar { value; from; to; indeterminate }` from
  QtQuick.Controls 2.
- WPF / WinUI: `<ProgressBar Minimum Maximum Value IsIndeterminate/>`.

**What composition loses:**

- A11y role (`role="progressbar"` /
  `AccessibilityRole.progressIndicator`) and the screen-reader
  progress announcement.
- Indeterminate animation: every platform ships its own
  marching-ant / spinner shape for unknown-duration work. Hand-
  rolling it gets the timing and accessibility wrong.
- Theming: progress bars are heavily themed (macOS striped vs.
  Windows solid vs. iOS thin). Composition can't follow OS
  theme automatically.

**Verdict:** ✅ Accept. The userland `Spinner` covers the
indeterminate-animation case but loses the role and the
determinate-progress case entirely.

**Proposed slot/emit surface:**

| moslayout prop  | Kind        | Required | Meaning                                       |
|---|---|---|---|
| `value`         | slot ref    | no       | current value (omit → indeterminate)          |
| `max`           | num literal | no       | progress total (default 100)                  |
| `indeterminate` | slot/kw     | no       | force-indeterminate even when `value` is set  |

No emits — `HostProgress` is purely a display widget. The host
drives `value` from its own state.

## 4. Recommended next batch

**`HostSlider` and `HostProgress` together, in a single batch
(UI29-3).** Both are smaller surfaces than HostCheckbox/HostRadio
(no group coordination, no tri-state) and follow the established
"thin native-widget wrapper" pattern. Together they bring the
kernel to **20 primitives**.

`HostSelect` is **deferred to UI29-4.** It's accepted as a kernel
candidate but the `options: list of {value, label}` shape needs
moslayout grammar work that doesn't exist yet (the kernel currently
has no `list of record` slot type — only `list of <primitive>`).
Bringing the grammar up first, then `HostSelect`, gives a clean two-
PR sequence. Tracking it as UI29-4 keeps UI29-3 small.

## 5. Implementation roadmap (when UI29-3 is greenlit)

Following the UI29-2 template (one PR per stage):

- **U29-3-G/R** — add `HostSlider` and `HostProgress` to
  `moslayout-compiler::PRIMITIVES` + `mosaic-package-resolver::
  KERNEL_PRIMITIVES`. Tests pin the names. Kernel jumps to 20.
- **U29-3-K-react** — `<input type="range">` and `<progress>`
  lowering. New tests cover value/min/max/step/disabled/onChange/
  onCommit (slider) and value/max/indeterminate (progress).
- **U29-3-K-swiftui** — `Slider(...)` and `ProgressView(...)`.
- **U29-3-K-qt** — `Slider { ... }` and `ProgressBar { ... }`.
- **U29-3-K-html** — `<input type="range">` and `<progress>`
  (static-HTML, with data-* hooks for the slider's emits).
- **U29-3-K-webcomp** — same shapes in shadow DOM with inline
  handlers.
- **U29-3-K-xaml** — `<Slider/>` and `<ProgressBar/>`.
- **U29-3-P** — add `Slider` and `Progress` userland components to
  `mosaic-pkg-toolkit` v0.4 (thin wrappers; `Spinner`'s
  indeterminate animation gets factored on top of `HostProgress`
  with the indeterminate slot fixed).

Expected total: 8 PRs over ~5-8 hours of fresh-context work,
mirroring the UI29-2 cycle that just shipped.

## 6. Open questions for the next session

1. **`HostSelect` grammar prereq.** Specify the `list of {value:
   text, label: text}` slot type as part of UI29-4 — what's the
   minimal moslayout grammar addition? `slot foo : list of { ... }`?
   Or pass a list of `text` and a parallel list of labels?
2. **Slider orientation.** Vertical sliders exist on every backend
   (Qt's `orientation: Qt.Vertical`, etc.) but are rare in app UI.
   Defer to v2 — v1 ships horizontal only.
3. **Progress indeterminate animation control.** Some hosts want
   the animation to pause when the window blurs. Defer to v2;
   v1 keeps it always-on.
4. **Should `HostSpinner` be promoted too?** Could be a separate
   primitive for the indeterminate-only case, but `HostProgress`
   without `value` covers it on every backend. The userland
   `Spinner` becomes a thin wrapper over
   `HostProgress (indeterminate: true)`. Decision: don't promote
   `HostSpinner`; reuse `HostProgress`.

## 7. Out of scope

- The full per-backend specs (UI29-3 §3.1-§3.6 mirroring the
  UI29-2 spec template). Each implementation PR ships its own
  per-backend spec alongside the code, matching the UI29-2 pattern.
- The grammar additions needed for `HostSelect` (deferred to
  UI29-4 spec).
- Any breaking changes to existing primitives. UI29-3 is purely
  additive.

---

**Reviewer checklist:**

- [ ] Do `HostSlider` and `HostProgress` meet the UI29 §2.2
      inclusion criteria? (Author argues yes — see §3.2 and §3.3.)
- [ ] Is the `HostSelect` deferral acceptable, or should the
      grammar work be folded into UI29-3?
- [ ] Is the proposed slot/emit surface for each primitive minimal
      enough? (Goal: no slot that *every* backend would have to
      ignore.)
