### Added - native indeterminate checkbox state (#13006)

`emit_host_checkbox`'s `indeterminate:` prop was accepted but ignored —
the doc comment even named Flutter's own `Checkbox(tristate: true)`
mode as the mechanism, but nothing in the function ever read the prop.
When `indeterminate:` is authored as anything other than a literal
`Keyword("false")`, the emitter now adds `tristate: true` and collapses
`value:` to `{indeterminate} ? null : {checked}` (Flutter's own
"`null` renders the dash" convention). No changes were needed to the
`onChanged` wiring: `Checkbox.onChanged` is always `void
Function(bool?)` even in two-state mode, and the existing
`host_checkbox_event_args` already treats its `v` parameter as
nullable (`v ?? false`).

A `slot:`/expression-valued `indeterminate` can't be evaluated at
compile time, so — mirroring XAML's `IsThreeState` treatment — its mere
*presence* unconditionally enables `tristate: true`; the runtime value
decides whether `null` (the mixed dash) actually renders.

New `pub fn host_checkbox_has_native_semantics` lets
`mosaic-package-artifact-builder`'s degradation analyzer stop reporting
`property.checkbox-indeterminate-ignored` for Flutter wherever this
lowering actually applies.

Verified against a real regenerated `mosaic-pkg-toolkit` project:
`flutter pub get` + `flutter analyze` — "No issues found!" — on the
whole package, including the real `Checkbox.dart` this change touches.

