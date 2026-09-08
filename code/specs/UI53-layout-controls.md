# UI53: Reusable Browser Form Controls

## Status

Implemented by `layout-controls`, produced by browser adapters, and consumed by
shared layout, paint, hit-testing, and host input pipelines.

## Contract

`LayoutNode.ext["control"]` carries a stable preorder-and-ID control key, form
name, kind, current value, placeholder, options, selected index, intrinsic
columns/rows, disabled/read-only/required/checked/focused state, and
computed appearance. The package resolves deterministic intrinsic sizes for
text inputs, buttons, textareas, selects, checkboxes, and radio buttons.

The contract contains no HTML parser, CSS parser, paint backend, or native
toolkit dependency. HTML and CSS adapters project authored state into it;
layout and paint consumers read the same metadata; browser sessions own state
transitions and expose semantic pointer and keyboard events to platform hosts.

## Invariants

- Explicit CSS width and height override intrinsic dimensions independently.
- Intrinsic dimensions remain identical in block, inline, flex, grid, table,
  positioned, and floated formatting contexts.
- Disabled controls never focus or mutate, and read-only text controls can
  focus without accepting edits.
- Password values never enter paint text; only an equal-length mask does.
- Hit regions use final positioned geometry and the same clipping rules as
  links.
- Unknown metadata is ignored and malformed required fields produce bounded
  diagnostics rather than backend-specific fallback behavior.
- Duplicate IDs cannot collide because the stable key always includes the
  control's retained-tree preorder index.
