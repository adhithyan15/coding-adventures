### Fixed — a conditional Row child holding a single `Text` never got its `Flexible` (#14857)

The wrap was gated on `if_node.children.len() > 1`, i.e. only a branch the
emitter renders as a `Column`. `If ( when: .. ) { Text .. }` is a single-child
branch, so it matched neither that rule nor the plain-child rule beside it,
which keys on `child.tag == "Text"` — the conditional's tag is `If`. The `Text`
measured at its natural width and the Row overflowed.

Replaced the child-count test with `branch_can_shrink`, which asks whether a
branch can give width back: a multi-widget branch (a `Column`, the original
case) or a single `Text`. Anything else — an `Icon`, a sized `Box` — is left
alone; a fixed-size widget should not be handed a loose constraint it never
asked for. An absent `else` renders as `const SizedBox.shrink()` and is
excluded from the decision, being already zero-width.

Measured on TaskApp's generated Flutter project, same clean state both runs:

| | `Flexible` wraps | conformance test |
| --- | --- | --- |
| before | 2 | `A RenderFlex overflowed by 316 pixels` — fails |
| after | 25 | passes |

The task row is five of these conditionals side by side, which is why it
overflowed by a figure that did not move across three attempts at the
surrounding layout: none of them had touched the children that were actually
overflowing. Reading the generated Dart showed the row contained zero
`Flexible` wraps and answered in one step what the layout reasoning could not.

**This does not change** how a plain (non-conditional) Row child is treated,
nor the nested-Row case, where a loose `Flexible` remains invalid because the
parent shrink-wraps its child: `multi_widget_if_inside_nested_row_does_not_emit_flexible`
still holds.

