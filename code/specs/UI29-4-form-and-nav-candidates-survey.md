# UI29-4 — Next kernel-promotion batch (post-UI29-3) survey

> **Status.** Survey + spec draft (no implementation).
>
> **Parent.** UI29 — Primitive Kernel + Userland Component Packages
> (`code/specs/UI29-primitive-kernel.md`).
>
> **Predecessors.** UI29-1 (`HostDialog`, kernel #16); UI29-2
> (`HostCheckbox` + `HostRadio`, kernel #17/#18); UI29-3
> (`HostSlider` + `HostProgress`, kernel #19/#20 — survey already
> merged at `code/specs/UI29-3-form-controls-survey.md`).
>
> **Scope.** Audits the existing `mosaic-pkg-toolkit` for *remaining*
> fake-HostButton (and other fake-X) patterns, then evaluates a
> fourth batch of kernel-promotion candidates against the UI29 §2.2
> criteria. Same "spec only" shape as the UI29-3 survey.

---

## 1. Remaining fake-X patterns in mosaic-pkg-toolkit (post-UI29-2)

After UI29-2's Checkbox/Radio fix, the toolkit still has the
following components composing from a primitive that loses
platform-native semantics:

| Toolkit component | Fake-X shape today                                    | What's lost |
|---|---|---|
| `Breadcrumb`      | `For { HostButton[breadcrumb-link] }`                 | a11y `role="link"` / `aria-current="page"`, Ctrl-click new-tab, middle-click new-tab, right-click context menu with "Open in new tab" / "Copy link", visited-state styling, screen-reader "link" announcement vs "button" |
| `Nav`             | `For { HostButton[nav-link] }`                        | same as Breadcrumb |
| `Toast.close-btn` | `HostButton[toast-close-btn]` with `label: "x"`       | (none — a close icon-button IS a button; not actually a fake-X. Annotated here only because the same component category — "icon button" — is one HostTooltip would help) |
| `Spinner`         | `Stack { Icon[spinner-glyph] }`                       | (none, but a candidate to reroute through `HostProgress (indeterminate: true)` once UI29-3 lands — covered in that spec's §6 Open Question 4) |
| `Field` error message | (inline composition, no fake-X)                    | none |

The only true fake-X pattern remaining is **Breadcrumb + Nav both
faking `<a href=...>` via `HostButton`**. That maps cleanly to a
single new kernel candidate: `HostLink`.

## 2. The candidates

This survey looks at four. Each is currently *missing* from the
kernel; each either has a userland equivalent already (the
fake-HostButton pattern surveyed above) or is a frequent-enough
need that promoting it would substantially simplify userland
toolkit composition.

| Candidate          | Native equivalent on every backend?                          |
|---|---|
| `HostLink`         | `<a href>` / `Link(destination:)` / `Text { … MouseArea }` (Qt — no native hyperlink) / `<Hyperlink>` (XAML) / `InkWell` + `url_launcher` (Flutter) |
| `HostTooltip`      | `title="..."` attribute + custom `<div>` / `.help(_)` / `ToolTip.text` / `<ToolTipService.ToolTip>` / `Tooltip(message:)` (Flutter) |
| `HostNumberInput`  | `<input type="number">` / `TextField` w/ numeric formatter / `SpinBox` (Qt) / `<NumberBox/>` (WinUI) / `TextField` + `keyboardType: TextInputType.number` (Flutter) |
| `HostMenu` (revisit) | `<menu>` + `<menuitem>` (still poor support) / `Menu(...)` (SwiftUI) / `Menu { MenuItem }` (Qt) / `<MenuFlyout>` (WinUI) / `PopupMenuButton` (Flutter) |

Three further candidates considered and rejected (or deferred to
UI29-5+):

- **`HostNumberInput` upper-bound** — could be promoted, but its
  surface largely overlaps with `HostSlider` (numeric with min/
  max/step). A `HostNumberInput` is appropriate where the user
  enters an *exact* value rather than dragging through a range
  (e.g. a year field). Worth promoting — see §3.3.
- **`HostFormButton`** (submit-only button) — every platform's
  form-submit button is just a styled button; no semantics lost.
  Stays userland.
- **`HostListItem`** — discussed but rejected: lists are too
  varied (selectable, expandable, drag-to-reorder, etc.) to fit
  a single primitive. Better as userland composition.

## 3. Per-candidate evaluation

### 3.1 `HostLink` — hyperlink / anchor

**Native widgets:**

- DOM: `<a href="..." target="_blank" rel="noopener noreferrer">`.
- SwiftUI: `Link("label", destination: URL(string: "..."))` (iOS 14+
  / macOS 11+), with platform-correct tap-to-open behaviour.
- Qt: there is no first-class hyperlink control. The idiomatic
  shape is `Text { textFormat: Text.RichText; text: "<a
  href='...'>label</a>"; onLinkActivated: Qt.openUrlExternally(link) }`.
- WPF / WinUI: `<Hyperlink NavigateUri="...">label</Hyperlink>`
  (wraps inside a `TextBlock` or as a `RichTextBlock` Inline).
- Flutter: `InkWell(onTap: () => launchUrl(uri), child: Text(...))`
  with the `url_launcher` package; or `RichText` with `WidgetSpan`
  containing the tap region. The `url_launcher` add-on is universal
  enough to count as native.

**What composition loses (`HostButton` faking a link):**

- A11y role (`role="link"` vs `role="button"`) and the screen-
  reader "link" / "visited link" announcement.
- Browser-level keyboard semantics: Tab to focus, Enter to
  activate, Ctrl/Cmd-click to open in new tab, middle-click to
  open in new tab in a background tab, right-click for the
  "Copy link" context menu, drag-to-bookmark.
- `visited` state styling — browsers track visited URLs and the
  `:visited` pseudo-class lets the link change colour, which is a
  user-expected affordance for navigation history.
- Form-submit semantics: a `<button>` inside a `<form>` submits;
  an `<a>` doesn't. Conflating the two breaks one or the other
  per use.

**Verdict:** ✅ **Accept.** Strongest case of the four —
Breadcrumb and Nav in the toolkit *already need it today*, and
the platform-specific shape varies enough across backends
(especially Qt's lack of a first-class hyperlink control) that
userland composition is provably impossible.

**Proposed slot/emit surface:**

| moslayout prop  | Kind        | Required | Meaning                                       |
|---|---|---|---|
| `href`          | slot/string | yes      | URL or in-app route. Resolved verbatim by the backend (HTML uses `href=`, SwiftUI parses into `URL`, etc.) |
| `label`         | slot/string | no       | Visible text. When omitted, children render in place. |
| `target`        | keyword     | no       | `same` (default) / `new-tab` / `parent` / `top`. Maps to DOM `target=` and Flutter/SwiftUI/Qt's URL-launch mode. |
| `external`      | keyword     | no       | `true` (default for `http(s)://` hrefs) / `false`. Backends that route between an in-app router and an OS browser use this to decide. |
| `onActivate`    | emit        | no       | Fires when the link is followed. Payload: `{href: string}`. Lets the host log analytics / preventDefault for in-app routing. |

### 3.2 `HostTooltip` — hover-help annotation

**Native widgets:**

- DOM: `title=` attribute on any element (lightweight, no styling
  control) plus `aria-describedby` for accessibility. Richer
  visuals require a userland popover.
- SwiftUI: `.help("label")` view modifier (macOS, iOS 16+).
- Qt: `ToolTip.text: "label"` on any QtQuick.Controls 2 widget,
  or `ToolTip { ... }` standalone.
- WPF / WinUI: `<element ToolTipService.ToolTip="text"/>` attached
  property; richer visuals via `<ToolTip><StackPanel>...</StackPanel></ToolTip>`.
- Flutter: `Tooltip(message: "text", child: ...)` widget — wraps
  any child with hover/long-press triggered tooltip.

**What composition loses (custom-built tooltip):**

- A11y wiring (`aria-describedby` linking the host element to the
  tooltip text) and the screen-reader announcement on focus.
- Touch-vs-mouse trigger heuristics: on mobile a long-press shows
  the tooltip; on desktop hover. Every platform has its own
  policy here.
- Z-order / clipping: tooltips need to escape their parent's
  overflow:hidden and z-index stack. Reimplementing requires
  portal-style mounting on web, popup-window plumbing on
  SwiftUI/Qt — all things the native widget already does.

**Verdict:** ✅ **Accept** but with a caveat — the DOM's `title=`
attribute is much less expressive than the other platforms'
tooltips (no styling, no rich content, browser-controlled timing).
For v1, scope HostTooltip to *plain-text tooltips* only — every
platform has a clean path for that. Rich-content tooltips
(images, multiline, formatted) are reserved for UI29-5+.

**Proposed slot/emit surface:**

| moslayout prop  | Kind        | Required | Meaning                                       |
|---|---|---|---|
| `text`          | slot/string | yes      | The tooltip body. Plain text only in v1.      |
| `target`        | (child)     | yes      | The element the tooltip annotates. Passed as the single child of HostTooltip. |
| (no emits)      |             |          | Tooltips don't dispatch — they're display-only. |

The "child as target" shape is unusual for a kernel primitive
(most primitives are leaves or container-with-many-children).
But it matches every backend's idiom exactly: Flutter's `Tooltip
(message, child)`, XAML's attached property on a child, SwiftUI's
`.help` modifier on a view. The HTML lowering wraps the single
child in a `<span title="text">…</span>` (or sets the attribute
on the child directly when the child is a single element — a
backend-specific optimisation).

### 3.3 `HostNumberInput` — numeric entry field

**Native widgets:**

- DOM: `<input type="number" min max step value>` with browser-
  enforced numeric-only entry + mobile numeric keyboard.
- SwiftUI: `TextField("...", value: $n, format: .number)` (iOS
  15+) or `Stepper` for ±-button entry.
- Qt: `SpinBox { from; to; stepSize; value }` from QtQuick.Controls
  2 — includes ± buttons and direct text entry.
- WPF / WinUI: `<NumberBox Minimum Maximum Value SmallChange/>`
  (WinUI 3 only; WPF needs a third-party control or a TextBox
  with validation).
- Flutter: `TextField(keyboardType: TextInputType.number, inputFormatters: [FilteringTextInputFormatter.digitsOnly])`.

**What composition loses (`HostInput` + manual validation):**

- Numeric-only keyboard on mobile (HTML's `inputmode="numeric"` +
  type="number"; Flutter's `keyboardType`).
- ± stepper buttons (Qt SpinBox, WinUI NumberBox) — composed
  versions force the user to focus the field and type.
- Up/Down arrow-key increment by step (HTML's built-in behavior).
- Decimal-format locale awareness (some browsers parse `1,5` as
  1.5 in fr-FR; HostInput drops this).
- Numeric-range validation: the platform reports a validation
  error if `value` is out of `[min, max]`. Reimplementing
  requires a custom error UI per backend.

**Verdict:** ✅ **Accept.** Smaller case than HostLink/HostTooltip
but still clear — the mobile-keyboard alone is worth promotion.

**Proposed slot/emit surface:**

| moslayout prop  | Kind        | Required | Meaning                                          |
|---|---|---|---|
| `value`         | slot ref    | yes      | numeric value                                    |
| `min`           | num literal | no       | minimum (compile-time)                           |
| `max`           | num literal | no       | maximum                                          |
| `step`          | num literal | no       | step increment (default 1; 0.01 for decimal)     |
| `placeholder`   | slot/string | no       | placeholder text shown when empty                |
| `disabled`      | slot/kw     | no       |                                                  |
| `onChange`      | emit        | no       | fires on commit (Enter / blur); `value: number` |

`onChange` semantics mirror HostInput's `onCommit` (not its
continuous `onChange`) — numeric fields don't typically want to
dispatch per-keystroke since intermediate states ("12" while
typing "12.5") aren't meaningful values.

### 3.4 `HostMenu` (revisit from UI29-3 rejection)

**Native widgets:**

- DOM: `<menu>` + `<menuitem>` (poor browser support; the de-facto
  shape is a custom `<ul role="menu">` + `<li role="menuitem">`).
- SwiftUI: `Menu("Title") { Button(...) Button(...) }` (iOS 14+).
- Qt: `Menu { MenuItem { text: "..." }; MenuItem { ... } }`.
- WPF / WinUI: `<MenuFlyout><MenuFlyoutItem/></MenuFlyout>` attached
  to a button; or `<MenuBar>` for app menus.
- Flutter: `PopupMenuButton<T>(itemBuilder: ...)`.

**What composition loses:**

- A11y role (`role="menu"` + `role="menuitem"` + `aria-haspopup`)
  and the screen-reader "menu" announcement.
- Keyboard navigation: arrow keys move focus between items,
  Enter activates, Escape closes, character keys quick-jump to
  matching item. Composed menus get this wrong constantly.
- Auto-positioning: native menus reposition themselves to stay
  in viewport (flip above the trigger when there's no room below,
  etc.) — composed popovers don't.
- OS integration: on macOS, native menus integrate with the
  menubar's keyboard shortcuts and the system's "Help → Search"
  feature.

**UI29-3 rejection rationale revisited:** the UI29-3 spec said
"interactions vary too much across platforms (menubars vs.
context menus vs. action sheets) for a single cross-backend
primitive". Reconsidering: a *single-style* `HostMenu` —
specifically the "click a button, popover appears with a list of
selectable items" shape — does have a 1-for-1 native widget on
every backend (PopupMenuButton on Flutter, MenuFlyout on WinUI,
Menu on SwiftUI/Qt, custom-but-standard pattern on web). The
divergence the UI29-3 rationale flagged is between *this kind of
menu* and *menubars*; restricting `HostMenu`'s scope to the
popover form gives a clean primitive.

**Verdict:** ✅ **Accept** with the scope restricted to *popover
menus only*. Menubars are intentionally out of scope (they require
OS-level integration that varies wildly — macOS's global menubar
vs. Windows' window-local menubars).

**Proposed slot/emit surface:**

| moslayout prop  | Kind        | Required | Meaning                                       |
|---|---|---|---|
| `items`         | slot ref    | yes      | list of `{label: text, value: text}` records (same `list-of-record` shape as the deferred `HostSelect`) |
| `trigger-label` | slot/string | no       | text shown on the trigger button when the menu is closed |
| `disabled`      | slot/kw     | no       |                                                |
| `onSelect`      | emit        | yes      | payload `{value: text}`                       |

`HostMenu` shares the `list-of-record` slot type with `HostSelect`
(see UI29-3 §3.1, deferred to UI29-4 grammar work). So this
candidate and HostSelect should land *together* in a single batch
that includes the grammar prereq.

## 4. Recommended batches

Given the grammar prereq for the `list-of-record` slot type that
both `HostSelect` (deferred from UI29-3) and `HostMenu` need:

### UI29-4 batch — no grammar dependency

- **`HostLink`** → kernel primitive #21
- **`HostTooltip`** → kernel primitive #22
- **`HostNumberInput`** → kernel primitive #23

All three follow the established "thin native-widget wrapper"
pattern and have surfaces that fit existing moslayout grammar
(slot ref, string literal, keyword, emit ref). 9-PR cycle
mirroring UI29-2's: G/R + six K-* backends + P (toolkit
rewrites — Breadcrumb and Nav rebuild on `HostLink`; a new
`Tooltip` userland component lands as a thin wrapper).

### UI29-5 batch — needs grammar work first

- **Grammar prereq:** add `list of {key: type, key2: type2, ...}`
  record-typed slot to moslayout.
- **`HostSelect`** → kernel primitive #24 (from UI29-3 deferral)
- **`HostMenu`** → kernel primitive #25 (from §3.4 above)

Two-spec sequence so the grammar work lands first and the two
primitives can share the same slot-type infrastructure.

## 5. Implementation roadmap (UI29-4 — when greenlit)

Following the now-well-established UI29-2/3 template:

- **U29-4-G/R** — add `HostLink`, `HostTooltip`, `HostNumberInput`
  to the `PRIMITIVES` + `KERNEL_PRIMITIVES` rosters. Kernel
  jumps from 20 → 23. (Assumes UI29-3's #19/#20 have landed first.)
- **U29-4-K-react** — `<a href>`, `<span title>`, `<input type="number">`
- **U29-4-K-swiftui** — `Link(...)`, `.help(...)`, `TextField` w/ `.number` format
- **U29-4-K-qt** — `Text` w/ rich-text anchor handler, `ToolTip.text`, `SpinBox`
- **U29-4-K-html** — same as React (static HTML)
- **U29-4-K-webcomp** — same shapes in shadow DOM
- **U29-4-K-xaml** — `<Hyperlink>`, `ToolTipService.ToolTip`, `<NumberBox>`
- **U29-4-K-flutter** — `InkWell` + `url_launcher`, `Tooltip(...)`, `TextField` w/ numeric keyboardType (7th backend now wired since the Flutter PR)
- **U29-4-P** — `mosaic-pkg-toolkit` v0.4:
  - Rewrite `Breadcrumb` and `Nav` on `HostLink` (closing the
    only remaining fake-X pattern in the toolkit)
  - New `Tooltip` userland component (thin HostTooltip wrapper)
  - New `NumberInput` userland component
  - Spinner gets a follow-up issue to consider rerouting through
    HostProgress once UI29-3 lands

## 6. Open questions for the implementation PRs

1. **HostLink security.** The DOM lowering needs to default
   `target="_blank"` links to `rel="noopener noreferrer"` so
   they can't reverse-tabnab the opener. Same gotcha exists in
   other backends (Flutter's `url_launcher` is fine; Qt's
   `Qt.openUrlExternally` always uses the OS handler; XAML
   doesn't share an opener). Worth documenting in the K-react
   and K-html specs.
2. **HostLink in-app routing.** The `external: false` case
   should suppress the default navigation and dispatch
   `onActivate({href})` so the host's router takes over. Spec
   the exact `preventDefault` / `event.preventDefault()` shape
   per backend.
3. **HostTooltip rich content.** Out of scope for v1 — spec a
   v2 add that lets `children` be the tooltip content instead
   of the `text:` slot.
4. **HostNumberInput vs. HostInput overlap.** The two primitives
   share `placeholder` / `disabled` / `onChange`. Worth
   collapsing into a single `HostInput` with a `type: number`
   keyword? Decision: NO — backends like SwiftUI use different
   widgets (`TextField` vs `TextField + formatter`) and the
   `min`/`max`/`step` props are meaningful for number only. Keep
   separate.
5. **Should `HostMenu` v1 be scoped to "button-triggered" only,
   or also support "context menu" (right-click)?** Lean toward
   button-triggered only — right-click context menus have weird
   accessibility / discoverability issues on touch devices.
   Context menu = UI29-5.1 follow-up.

## 7. Out of scope

- The grammar additions needed for `list of {record}` slot types
  (covered by the UI29-5 grammar-prereq spec).
- `HostSelect` and `HostMenu` themselves (deferred to UI29-5).
- Rich-content tooltips (UI29-5+).
- Menubars (out of scope for the kernel; userland composition
  on top of `HostMenu` if a host wants menubar-style behaviour).
- Touch-context-menu styles (mobile long-press menus that mimic
  iOS's action sheets / Android's contextual actions).

---

**Reviewer checklist:**

- [ ] Do `HostLink`, `HostTooltip`, `HostNumberInput` meet the
      UI29 §2.2 inclusion criteria? (Author argues yes — see
      §3.1, §3.2, §3.3.)
- [ ] Is splitting UI29-4 (no-grammar) from UI29-5 (grammar
      prereq) the right ordering, given the dependency?
- [ ] Is the `external: true / false` keyword on `HostLink` the
      right axis for the in-app-vs-OS-browser decision, or
      should it default to "always dispatch onActivate and let
      the host route"?
