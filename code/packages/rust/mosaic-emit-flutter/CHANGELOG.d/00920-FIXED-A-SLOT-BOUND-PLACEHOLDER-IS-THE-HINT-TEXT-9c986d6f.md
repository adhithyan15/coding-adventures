### Fixed — a slot-bound placeholder is the hint text

`placeholder : slot: x` on `HostInput` and the legacy `Input` was dropped:
the hint was read with a string-literal-only lookup, so every toolkit `Input`
and the note-type editor's fields had no `hintText`, and the degradation
report said nothing. `host_input_hint_expr` now reads every authored form: a
literal or number as an escaped Dart string, a slot or `For` binding as its
validated camelCase name, an expression verbatim (as `a11y-label` does). It
still feeds the one `InputDecoration` producer, `host_input_decoration_arg`,
which now takes the hint as a ready Dart expression.
