### Added - native radio-group mutual exclusion (#13007)

`emit_host_radio`'s `group:` prop only fed a per-radio-independent
`groupValue` that defaulted to `null` for a literal string group value
(discarding it entirely) — real usage's radios never actually rendered
as selected, let alone exclusively. A literal `group: "..."` value
shared by 2+ `HostRadio`s with a resolvable `checked:` prop anywhere in
the component (see `collect_radio_group_members`) now gets a
synthesized shared `groupValue`: the SAME nested-ternary Dart
expression (`cond1 ? val1 : (cond2 ? val2 : (... : null))`) emitted at
every member's own call site. Flutter's `Radio<T>` computes its checked
state as `value == groupValue`, so sharing one reactive expression
across siblings gives real native mutual exclusion with zero
restructuring of the widget tree — no ancestor wrapper needed, and
`onChanged` is untouched (each radio still dispatches its own
`onSelect` independently).

Deliberately uses the classic `groupValue`/`onChanged` API rather than
Flutter's newer `RadioGroup<T>` ancestor widget, which requires Flutter
3.35+ (this repo's pinned floor is 3.32) — the classic API remains
fully supported and achieves the identical exclusivity guarantee with
no SDK bump.

Threaded via a new `TableCtx.radio_group_members` field, computed once
in `emit_widget_class` and inherited unchanged through every recursive
call. A `slot:`-bound group, or a literal value with no qualifying
peer, keeps the pre-#13007 behavior exactly (including its existing
`groupValue: null`-always-unselected gap for those cases).

New `pub fn radio_groups_with_native_semantics` lets
`mosaic-package-artifact-builder`'s degradation analyzer stop reporting
`property.radio-group-ignored` wherever this lowering actually applies.

Verified against a real regenerated `mosaic-pkg-deck-options` project
(the real multi-radio usage this targets): `flutter pub get` +
`flutter analyze` — "No issues found!".

