# XAML repeated-cell visual-state predicates

Related: #14274 and epic #14267.

The XAML emitter drops compound state-when expressions inside For templates even though its expression lowerer now projects helpers onto typed row view models. Reuse that projection for compound visual-state predicates without emitting page-method calls in template markup. Keep unsupported expressions omitted and preserve template-local VisualStateManager groups.

Track component-slot dependencies of supported predicates on the outer row projection so selection changes reconstruct immutable nested row contexts. Match parsed identifier tokens to declared slots, respecting lexical loop bindings; never splice raw expressions into generated markup. Preserve simple selected-index projections.

Validate generic compound predicates, nested captured indices, projection invalidation, and template-local bindings. Build and run the generated VisiCalc Windows app with the standard Rust adapter and an isolated state fixture. Verify initial selection, movement, and formula commit visually. Do not patch generated output or claim full native/accessibility/persistence acceptance.

## Native validation, 2026-10-05

The first implementation compiled but crashed in generated
Set_Microsoft_UI_Xaml_StateTrigger_IsActive during template ProcessBindings.
The final emitter uses a UserControl/Grid state host and collapsed named Border
Tag proxies for compiled predicates. Triggers bind to those dependency properties
by template-local name; no direct StateTrigger x:Bind remains in repeated rows.

The generated project builds with zero errors and ten WMC1506 warnings (eight
existing immutable-row warnings plus two predicate proxy bindings). All 365
emitter unit/integration tests pass; one doctest is ignored.

Actual Windows interaction with the Rust adapter and an isolated fixture showed:
- A1's selected background is visible, with no startup crash.
- Committing =21+22 through the formula field produces A1=43, E1=66, A5=67,
  E5=197; the selected background persists and status reports onCommit.
- A separate restored B2 fixture highlights B2=14 instead of A1.
- Clicking New workbook resets the address to A1 and moves the highlight there;
  the old B2 background disappears, and status reports onNewWorkbook.

Clicking B2 in the populated A1 fixture did not change selection. Pointer/keyboard
cell movement remains an acceptance gap under #14274/#14276, as do formula-field
sizing, numeric text alignment, native accessibility, and full persistence checks.
This change establishes state rendering and refresh, not full native acceptance.
