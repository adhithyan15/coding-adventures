## 2026-09-27 (layout variants share one app)

- **A layout variant compiles beside the default (UI48 §7.5, ENV2).** `from_pipeline_variant(interface, layout, style, variant)` emits a variant's composable as `<Component><Variant>` (`EngramApp.touch.mll` → `fun EngramAppTouch(`), named by the new `variant_composable_name` (`-` and `_` separate words; anything else is refused). A variant file declares none of the component's interface -- no `<C>Event` sealed class, no `<C>Props` classes, no `Immutable` import -- and its composable takes and dispatches the default file's types, so the two no longer redeclare each other. Everything else a generated file declares at top level is `private`, and so file-scoped. `from_pipeline` is unchanged.

