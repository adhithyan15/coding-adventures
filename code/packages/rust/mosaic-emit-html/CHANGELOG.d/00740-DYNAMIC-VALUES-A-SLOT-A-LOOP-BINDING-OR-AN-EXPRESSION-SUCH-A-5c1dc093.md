- **Dynamic values** (a slot, a loop binding, or an expression such as
  `( i == selectedIndex )` inside `For`):
  - The emitter writes `data-mosaic&pressed="…"`, attribute-escaped and
    brace-escaped.
  - The project runtime's new `renderPressed` pass decides it with the same
    `evaluateCondition` that `mosaic-if` uses. The pass runs after loops and
    `If`s are expanded and before mustaches are filled.
  - It then replaces the attribute with `aria-pressed="true"` or `"false"`.
