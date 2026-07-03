# VisiCalc Grid — `Input` → `HostInput` migration audit

> **Status.** Audit complete; no migration needed at the `.mll`
> layer. Recommendations follow.
>
> **Parent.** `code/specs/visicalc-cross-backend-demo-plan.md`
> Phase 1 / VC1.6 (this audit) and the VisiCalc Phase 1
> migration that landed FormulaBar.

---

## 1. What the plan asked

VC1 (already shipped, #4078) migrated
`code/programs/mosaic/visicalc/FormulaBar.desktop.mll` from the legacy
`Input` primitive (UI25) to the kernel-canonical `HostInput`
(UI29 §2.1). The migration was a one-token rename with
byte-identical generated React output.

The plan asked us to do the same audit on `Grid.desktop.mll` and
migrate if the legacy `Input` primitive was referenced.

## 2. What the audit found

**`Grid.desktop.mll` does NOT reference the `Input` primitive.**

The full layout source (after the UI30 ML3 touch-variant work
landed alongside it):

```
layout Grid {
  Grid [sheet] (
    headers:       slot: column-headers,
    rows:          slot: viewport-rows,
    column-widths: slot: column-widths,
    selected-row:  slot: selected-row,
    selected-col:  slot: selected-col,
    edit-row:      slot: edit-row,
    edit-col:      slot: edit-col,
    sticky-header: true,
    total-height:  slot: total-height,
    onNavigate:    emit: onNavigate
  )
}
```

`Grid.touch.mll` (UI30 ML3) is structurally identical with
`sticky-header: false`. Neither file references `Input`.

The only primitive used is the built-in `Grid` itself (a slot-
driven table widget — different from `HostInput` etc.; it's the
big "render a viewport-windowed cell grid" primitive defined per
UI26 §3.1, not a UI29 host primitive). Cell editing is handled
internally by the React emitter's hardcoded `<input>` rendering
inside the editing-cell `<td>` (see
`code/packages/rust/mosaic-emit-react/src/pipeline.rs`'s Grid
arm) — that `<input>` lives in the **emitter**, not in the
`.mll`.

## 3. Conclusion

**No `.mll`-level migration is needed for Grid.** The audit task
in the VisiCalc Phase 2 plan is complete with no code change.

## 4. Follow-up: lower the emitter's hardcoded `<input>` through `HostInput`

The React emitter's Grid arm emits a literal `<input>` tag for
the editing cell. That's a violation of the UI29 spirit —
"every input the user types into should go through `HostInput`"
— but it's an emitter-internal concern, not a `.mll` author
concern. A future PR could:

1. Refactor the React Grid emitter to delegate its editing-cell
   `<input>` rendering to the same code path that `HostInput`
   uses (likely a small helper extraction in `pipeline.rs`).
2. Apply the same refactor to the other backends' Grid emitters
   once they exist (today only React lowers `Grid`; HTML /
   WebComponent / Flutter emit placeholders).

This isn't in scope for VisiCalc Phase 2 (the visual-demo
cycle); spec it as **UI31-grid-host-input-internal-refactor**
when the Grid emitter coverage broadens. The audit doc here is
the breadcrumb that records the finding so it isn't lost.

## 5. Why this audit shipped as a spec doc, not a code PR

The plan item asked "audit and migrate if needed." Migration
turned out to be unneeded at the `.mll` layer. Rather than ship
a no-op code PR or skip the plan item silently, we ship this
audit doc — it:

- Records what was checked and the conclusion (so a future
  reviewer doesn't redo the audit).
- Captures the deeper observation (emitter-internal `<input>`)
  that the migration question surfaces.
- Spawns a follow-up item with a concrete spec handle
  (UI31-grid-host-input-internal-refactor).

The plan advances cleanly: Phase 2 continues with VC2-qt next.
