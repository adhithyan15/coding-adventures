- **Qt pads Card's regions (UI21 §2.2 rule 6).** A styled `Box` now sizes
  itself on Qt from its content plus its per-edge padding, so
  `mosaic-pkg-card`'s `card-title` / `card-body` `padding-bottom` is no
  longer dropped there. Journal's Qt drop report falls from 7 to 3, and
  `(Qt, "padding-bottom")` left `CARD_STYLE_DROPS` in
  `tests/native_complete_gate.rs`; the inverse ratchet would otherwise
  fail on it. Flutter still drops it and keeps its pin.
