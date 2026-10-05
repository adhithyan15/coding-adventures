# XAML repeated-cell visual-state predicates

Related: #14274 and epic #14267.

The XAML emitter drops compound state-when expressions inside For templates even though its expression lowerer now projects helpers onto typed row view models. Reuse that projection for compound visual-state predicates without emitting page-method calls in template markup. Keep unsupported expressions omitted and preserve template-local VisualStateManager groups.

Track component-slot dependencies of supported predicates on the outer row projection so selection changes reconstruct immutable nested row contexts. Match parsed identifier tokens to declared slots, respecting lexical loop bindings; never splice raw expressions into generated markup. Preserve simple selected-index projections.

Validate generic compound predicates, nested captured indices, projection invalidation, and template-local bindings. Build and run the generated VisiCalc Windows app with the standard Rust adapter and an isolated state fixture. Verify initial selection, movement, and formula commit visually. Do not patch generated output or claim full native/accessibility/persistence acceptance.
