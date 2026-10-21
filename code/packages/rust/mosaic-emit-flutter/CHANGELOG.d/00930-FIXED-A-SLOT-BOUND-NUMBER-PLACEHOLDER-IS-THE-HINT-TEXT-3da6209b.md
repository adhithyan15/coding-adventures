### Fixed — a slot-bound number placeholder is the hint text

`HostNumberInput ( placeholder : slot: x )` was dropped (EM-14): the number
field read `placeholder` with a string-literal-only lookup, so
`mosaic-pkg-toolkit`'s `NumberInput` had no `hintText`, and the degradation
report said nothing. Its decoration now takes the hint from
`host_input_hint_expr`, as `HostInput` does: a literal or number as an
escaped Dart string, a slot or `For` binding as its validated camelCase name,
an expression verbatim. A name that is not a Dart identifier is an
`UnsafeSlotName` error.
