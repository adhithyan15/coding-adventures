---
category: Mosaic compiler pipeline
---

# A style-drop reporter that re-lowers properties through the container path cannot see a leaf writer that ignores them

**What went wrong.** On Compose, the `Text` arm in `mosaic-emit-compose`
computed the part's full `ComposeStyle` but passed only its *text* style
(colour, size, weight, family) to `emit_text`. The box chain was thrown
away: padding, background, rounded corners, size, border. Calendar's today
badge, a 21px amber pill, was never drawn, and Trestle's count and due-date
pills rendered as bare text.

The drop reporter (`dropped_style_properties_in_layout`) decides whether a
property is dropped by running it through `compose_box_style`, the
*container* lowering. Containers do lower `background`, so the reporter said
0 degradations and the strict `native-complete` profile passed. Nothing
flagged the loss until a screenshot showed a missing pill.

**Fix.** Thread `style.modifier` into `emit_text`, after the width decision
(fill, weight or the UI59 floor) and before semantics. That makes the
reporter's "handled" answer true again.

**Do differently.**
- A reporter that answers "is this dropped?" by re-running a *different*
  writer's lowering proves only that the property *could* be lowered. Check
  that every writer a part reaches consumes what the lowering produced.
- When a leaf writer takes a `ComposeStyle`, it should consume all of it, or
  report the fields it ignores.
- Look at renders. `native-complete: 0 degradations` is not the same as
  "nothing is lost".
