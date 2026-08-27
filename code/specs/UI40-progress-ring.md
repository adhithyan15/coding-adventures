# UI40 — `HostProgressRing`, a native determinate progress control

**Status:** kernel contract only (no backend renders it yet) — see §4.
**Kernel surface:** one new leaf primitive, `HostProgressRing`, added to
`moslayout-compiler::PRIMITIVES` and its mirror,
`mosaic-package-resolver::KERNEL_PRIMITIVES`.

---

## 1. The gap

TaskApp's workspace-progress ring (`task-app-icon-assets-v1.md`) is drawn as
a CSS-only donut trick:

```mll
Stack [ ring-circle ] {
  Box [ ring-fill ] ( background : slot: ring-gradient )
  Box [ ring-hole ] { }
}
```

`ring-fill`'s `background` is bound to a host-computed
`conic-gradient(...)` CSS string (UI36 data-driven sizing/styling) — a
filled circle whose fill *is* the progress arc, with a smaller
same-surface-colour circle stacked on top to punch the donut hole.
This is a real, working technique — on the web backend, whose native
box model has `conic-gradient`. Every other backend silently ignores
the slot-bound `background` on a plain `Box` (UI36's data-driven
`background` binding is a React-only mechanism), so today the ring
renders as nothing at all on every native backend.

A prior PR closed the *data* half of this gap: `TaskApp.mil`'s
`slot ring-percent-value : number ;` now carries the real 0..100
percent to every host (previously only the web host computed this
number, internally, in TypeScript). This spec closes the *rendering*
half.

## 2. Why this belongs in the kernel (UI29 §2.2)

1. **Every native platform ships a real determinate ring control.**
   WinUI has `ProgressRing`; SwiftUI has `ProgressView` (circular
   style); Compose has `CircularProgressIndicator(progress:)`; Flutter
   has `CircularProgressIndicator(value:)`. Qt/QML is the one gap — see
   §4.
2. **No reasonable composition exists on native backends.** The
   web's conic-gradient trick has no native-backend equivalent; there
   is no `Box`/`Stack` composition that reproduces a smooth circular
   percent-fill without either a real ring control or a `Path`-style
   drawing primitive (UI39) reproducing the arc by hand.
3. **The native accessibility surface matters.** A real
   `ProgressRing`/`CircularProgressIndicator` carries a `progressbar`
   role, an announced `aria-valuenow`-equivalent, and a value-changed
   live region for free — none of which a decorative `Box` composition
   (or even a hand-drawn `Path` arc) gets without extra semantics work.
   This is the same "native semantics a plain composition can't
   reproduce" argument `HostSlider` (UI29-3) and `HostSwitch` (UI38)
   already established for their own controls.

## 3. The primitive

A leaf primitive (no children, like `Icon`/`HostSlider`), added to
`moslayout-compiler::PRIMITIVES`. Deliberately minimal — this is a
**display-only, non-interactive** control (unlike `HostSlider`, which
is draggable): no `onChange`/`onCommit` emits, no `disabled` prop.

| Prop | Type | Required | Notes |
|---|---|---|---|
| `value` | `Number \| SlotRef \| Expr` | yes | The 0..100 percent. Full three-way binding from day one — reuses the same numeric-prop binding mechanism every other prop already has (`HostSlider`'s `value`, Gantt bar widths, `Path`'s coordinates once bound). A static ring would defeat the point: TaskApp's whole use case is `value: slot: ring-percent-value`. |
| `a11y-label` | `String \| SlotRef` | no | Matches `HostSlider`'s accessibility prop — every native ring control here has a real accessible-label/description surface. |

```mll
HostProgressRing [ ring-circle ] ( value: slot: ring-percent-value )
```

No new style properties. `width`/`height` (TaskApp's `ring-circle` part
is already 34×34) resolve through the same generic per-part style-map
lookup every primitive already gets — no kernel or `mosstyle` grammar
changes.

> **Non-goal.** `HostProgressRing` does not attempt pixel parity with
> the web backend's two-tone donut look — it renders each platform's
> own native ring, matching the epic's stated philosophy (UI29 /
> #12017): "components that look and feel native per platform, not
> identical layout across platforms." A `Path`-primitive-drawn arc
> (UI39's deferred `arc` kind) was considered and explicitly rejected
> for this use case — more work, and pixel-matching wasn't the goal.

## 4. Rollout

Sequenced like `HostSlider`/`Path`: one kernel-contract PR (this spec +
`PRIMITIVES`/`KERNEL_PRIMITIVES` registration + degradation plumbing +
a round-trip fixture — no rendering), then one PR per backend.

| Backend | Status |
|---|---|
| XAML | not yet — `<ProgressRing IsIndeterminate="False" Value="{n}" Minimum="0" Maximum="100"/>` is the target API (this repo already emits the indeterminate `<ProgressRing IsActive="True"/>` form for `Icon(glyph: "spinner")`); needs empirical `dotnet build` verification before implementation. |
| Flutter | not yet — `CircularProgressIndicator(value: fraction)` (0.0–1.0), wrapped in `SizedBox(width:, height:)` for explicit sizing (the widget itself has no size params); needs a `value/100.0` conversion. Same widget already used indeterminate (no `value` arg) for the Icon spinner case. |
| Compose | not yet — `androidx.compose.material.CircularProgressIndicator(progress: Float)` (0f–1f); same widget already used indeterminate for the Icon spinner. This repo pins Compose Material1, not Material3 — the determinate overload's exact signature needs empirical confirmation via a real `gradle compileKotlin` before implementation. |
| Qt | not yet, and the one real gap — Qt Quick Controls 2 has no out-of-the-box circular *determinate* ring (`ProgressBar` is linear; `BusyIndicator`, already used for the Icon spinner, is indeterminate-only with no `value` prop). Will likely need a custom QML delegate (an arc drawn via `Canvas`/`Shape`) or a third-party style — its own research spike, sequenced last among the four backends. |
| SwiftUI | not yet — tracked separately in #13206 (genuinely unbuildable on this dev box, no macOS/Xcode environment for real verification, same blocker as `Path`'s SwiftUI lowering). |
| react, html, webcomponent | not in scope for this cascade — TaskApp's shared `.mll` (one file across all 8 backends) keeps using the current `Box`+`Box` CSS trick, which still renders correctly on web today. Migrating TaskApp's actual markup to `HostProgressRing` is an explicit, separate, later slice, gated on either accepting a web gap (matching `HostSlider`'s own precedent — never implemented on the three web backends) or adding a web lowering first. Not decided now, to avoid regressing web's currently-working ring. |

Until a backend implements `HostProgressRing`, an author using it
renders nothing (a reported degradation, not silent) on that host — the
same posture UI39 §6 and UI36 §5 established for unimplemented
backends.
