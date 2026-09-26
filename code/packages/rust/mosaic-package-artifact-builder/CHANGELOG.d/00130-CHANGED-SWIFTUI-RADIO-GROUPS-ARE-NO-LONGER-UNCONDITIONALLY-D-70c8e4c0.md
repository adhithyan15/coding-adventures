### Changed — SwiftUI radio groups are no longer unconditionally degraded

`property.radio-group-ignored` now consults
`mosaic_emit_swiftui::pipeline::radio_groups_with_native_semantics`, alongside
the Qt, Compose and Flutter predicates it already used. SwiftUI lowers a
qualifying run of sibling radios to a `Picker` (#13007), so the degradation is
reported only for the groups it genuinely cannot resolve.

