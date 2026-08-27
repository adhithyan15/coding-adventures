# UI39 — `Path`, a kernel drawing primitive

**Status:** kernel contract only — no backend renders it yet. XAML is the
first backend targeted (this session, immediately following this spec).
**Kernel surface:** one new primitive, `Path`, added to the 33-entry kernel
list `moslayout-compiler::PRIMITIVES` (UI29 §2.1) and its mirror,
`mosaic-package-resolver::KERNEL_PRIMITIVES`.

---

## 1. The gap

Every existing "graphical" primitive is a catalog lookup, not real geometry.
`Icon` lowers to `FontIcon`/SF Symbols/Material Icons/a CSS icon-font class —
a name resolved against a platform-supplied glyph set, never drawn. Nothing
in the kernel can put an actual shape on screen.

`code/programs/mosaic/task-app` works around this with CSS techniques that
look fine on the web backend and vanish everywhere else:

```mss
part theme-toggle-moon {
  background : "#b3a99c" ;
  box-shadow : "5px -5px 0 0 #1a1714 inset" ;   ← this *is* the crescent
}
part pill-dot-ok {
  background : "currentColor" ;                  ← no XAML equivalent
}
```

Neither technique is a Mosaic bug being fixed — both are things CSS can do
that no native platform's box model can. SwiftUI's `.shadow()` has neither
spread nor inset; Compose has elevation tokens, not parameterised shadows;
`currentColor` is a CSS cascade keyword with nothing in XAML's binding model
to inherit from. `code/specs/task-app-icon-assets-v1.md` investigated this
exact gap and explicitly deferred it: "a much bigger, more speculative
kernel surface than anything else touched" — the six segmented-switch icons
were left unshipped for the same reason. The Gantt dependency-arrows item in
`code/programs/mosaic/task-app/BACKLOG.md` is blocked on the identical gap:
curved connector lines between two dynamically-positioned bars need "genuine
2D line-drawing the UI29 kernel has no primitive for."

## 2. Why this belongs in the kernel (UI29 §2.2)

1. **Every host platform has a native equivalent.** WinUI has `Ellipse`/
   `Line`/`Path`; SwiftUI has `Circle`/`Path`; Qt has `QtQuick.Shapes`;
   Flutter has `CustomPaint`; Compose has `Canvas`; every web backend has
   `<svg>`. This is not "can be simulated" — every platform ships a native
   vector-drawing surface; Mosaic's kernel has simply never exposed one.
2. **No reasonable composition exists.** A real circle cannot be built from
   `Box`/`Stack`/`border-radius` in a way that survives native lowering —
   that's the box-shadow-inset trick above, and it's exactly what breaks.
3. **It is semantically irreducible.** A drawn shape is not decoration
   layered on a box; it *is* the content (the moon, the connector line, the
   glyph). There is no fallback rendering that preserves meaning the way,
   say, an unstyled `<button>` still functions as a button.

## 3. The primitive

A leaf primitive — no children, like `Image`/`Spacer` — added to
`moslayout-compiler::PRIMITIVES`. Composed shapes (the crescent moon is two
overlapping circles) live inside a `Stack`, the same `position: relative`
layering `task-app-icon-assets-v1.md` already established for compositing
multiple parts, just with real geometry standing in for the CSS illusion.

### 3.1 Geometry — four `kind`s

`kind` is a `Keyword` prop, the same value shape `disabled: true`/
`align: row` already use. It selects which coordinate props apply:

| `kind` | Props | Covers |
|---|---|---|
| `circle` | `cx`, `cy`, `r` | the crescent moon (two overlapping circles), the status dot |
| `line` | `x1`, `y1`, `x2`, `y2` | segmented-switch icon glyphs (bars, grid lines) |
| `curve` | `x1`, `y1`, `cx`, `cy`, `x2`, `y2` — quadratic bézier: start, control, end | Gantt dependency connector arrows |
| `arc` | `cx`, `cy`, `r`, `start-angle`, `end-angle` | reserved for a future partial-ring use; specified now, not required by any current backend PR |

Every coordinate prop is `Number`-valued, and accepts the same
`Number | SlotRef | Expr` three-way binding every other numeric prop in the
kernel already has (Gantt bar widths, `ring-percent-value`, UI36 sizes) — no
new binding mechanism. A `Path` positioned dynamically (an arrow between two
data-dependent points, say) is therefore expressible with zero new grammar,
the same way a bound width already is.

```mll
Stack [ theme-toggle-moon ] {
  Path [ moon-disc ] ( kind: circle, cx: 17, cy: 17, r: 17 )
  Path [ moon-bite ] ( kind: circle, cx: 22, cy: 12, r: 17 )
}
```

### 3.2 Styling — zero new style properties

`Path` reuses two existing `.msl` properties rather than inventing a
stroke/fill vocabulary:

| Reused property | Means, for `Path` |
|---|---|
| `background` | fill colour |
| `border-color` / `border-width` | stroke colour / stroke width |

Style resolution is already generic by part name (`build_part_style_map` /
`part_styles.get(part_name)`) — no backend needs new style-resolution code,
and every existing style-degradation mechanism (`dropped_style_properties`,
tracked since #12022) keeps working for `Path` parts automatically, with no
changes to that machinery.

> **Non-goal.** `Path` does not accept an arbitrary SVG `d`-attribute path
> string, and does not accept a structured path-command list (mirroring
> `paint-instructions::PathCommand`'s `MoveTo`/`LineTo`/`CubicTo`/`ArcTo`
> vocabulary, which exists but powers an unrelated pipeline — see §5). Both
> were considered and rejected: a raw path string needs a hand-rolled
> SVG-mini-language parser in every backend without a built-in one (SwiftUI,
> Flutter — exactly the bigger, more speculative surface
> `task-app-icon-assets-v1.md` already declined to build), and a structured
> command list needs new grammar support for multi-field property lists
> that neither moslayout nor mosstyle has today. The four `kind`s above are
> chosen to cover every currently-known use case without either cost; if a
> fifth use case needs a shape none of the four express, extend the catalog
> rather than generalizing to a full path language.

## 4. Degradation reporting

`Path` follows `HostSwitch`'s and `HostSlider`'s lifecycle in
`mosaic-package-artifact-builder::collect_native_degradations`: registered
with an unconditional `"Path" if backend.is_native() => Some(("primitive.path-unimplemented", ...))`
arm the moment it's added to the kernel (every native backend reports it as
degraded, since none renders it yet), then narrowed with a
`!matches!(backend, Backend::Xaml | ...)` exclusion as each backend lands a
real lowering — XAML first, per §6.

## 5. Relationship to the existing paint pipeline

`mosaic-emit-paint` / `paint-instructions` / `barcode_2d` already implement
a complete SVG-equivalent vector vocabulary (`PathCommand::{MoveTo, LineTo,
QuadTo, CubicTo, ArcTo, Close}`, full stroke/fill/dash properties) — but it
operates on the **legacy single-file `.mosaic`/`MosaicNode`** front end via
`mosaic_analyzer::analyze()`, entirely bypassing the mosmodel/moslayout/
mosstyle kernel pipeline this spec's `Path` lives in, and never reaching any
of the 8 real backends (it only feeds a raster PNG export, UI22's Tier 2).
`Path` is a new, independent primitive on the kernel side; it does not
subsume or replace the paint pipeline's vocabulary. If `Path` ever grows
past the four kinds in §3.1 toward general path data, `PathCommand`'s field
shapes (in particular `ArcTo`'s `rx, ry, x_rotation, large_arc, sweep, x, y`)
are the reference to align with, since that vocabulary is already proven
against real SVG semantics — but that is out of scope here.

## 6. Rollout

Sequenced exactly like `HostSlider` (UI29-3): one kernel-contract PR (this
spec + the `PRIMITIVES`/`KERNEL_PRIMITIVES` registration + degradation
plumbing + a round-trip fixture — no rendering), then one PR per backend.

| Backend | Status |
|---|---|
| XAML | **this session, immediately following the kernel-contract PR** — `circle`/`line`/`curve` targeted; `arc` is a stretch goal, tracked separately if it slips |
| SwiftUI, Qt, Flutter, Compose | not yet — tracked as follow-up issues once XAML lands |
| react, html, webcomponent | not yet — no backend, native or web, renders `Path` until its own PR lands. (`HostSlider`'s rollout never reached the three web backends either; not repeating that gap silently — each remains an open, explicitly tracked issue rather than an assumed no-op.) |

Until a backend implements `Path`, an author using it renders nothing (a
reported degradation, not silent) on that host — the same posture UI36 §5
established for unimplemented backends.
