### Fixed — Qt screen regions size to their content

Every Engram screen region (`deck-options-region`, `review-region`,
`browser-region`, …) is a styled `Box` with no width or height. On Qt it
lowered to a `Rectangle` with no implicit size, so the layout gave it no
room and the content inside a bounded `content-scroll` measured almost
nothing: with EN-6 (#18477) and UI21 §2.2 rules 1–5 (#18481), the Options
screen's `ScrollView` held 104px of content in a 659px viewport and could
not scroll. UI21 §2.2 rule 6 now sizes a styled `Box` from its content
plus its per-edge padding and insets that content. Measured headless with
PySide6 6.12 at 1200×800: 1314px of content in the same 659px viewport,
scrollable, no binding loops.

The regions' `padding-bottom: 0` and `review-region`'s `padding-top: 16`
are now lowered, so Engram's Qt drop report falls from 38 to 22 (eight
per layout variant), and the `(Qt, "padding-bottom")` and
`(Qt, "padding-top")` pins left `ALLOWED_STYLE_DROPS` in
`tests/native_complete_gate.rs`.
