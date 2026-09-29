### Fixed — authored `opacity` reached nothing (#14708)

`opacity` was parsed and then dropped. The toolkit authors it on eight
controls — `state disabled { opacity : $opacity-disabled ; }` is what UI57's
disabled treatment is *made of* — so every disabled control on Flutter
rendered at full strength, looking enabled.

Flutter has no opacity argument: `Opacity` is a widget that **wraps** a
subtree. `emit_widget_tree` is therefore now a thin wrapper around
`emit_widget_tree_inner`, which applies the wrap once, on the way out. That
shape is deliberate — the real emitter has nine-plus `return` sites, and
wrapping each would have missed some silently. A test pins a child-only
opacity for exactly that reason.

- `StateLayer` gained an `opacity` field, populated where the other state
  properties are parsed, so state-layered opacity arrives the same way
  background and padding do.
- Each arm of the conditional renders through `dart_double_literal`
  (`... ? 0.4 : 1.0`). Dart accepts a bare int *literal* in a double context,
  so this is normalization rather than a fix — but a `num`-typed expression is
  not accepted, which is why the conversion is per-arm.
- Verified by `flutter analyze` on the generated toolkit project, not by
  inspection: 50 emitted Dart files, no issues.


