- `transportation/green-signal-permitted-movement.adj` (new) — a sibling to the already-shipped
  `traffic-lights.adj` (`traffic_light_meaning(color, meaning)`, green → the single atomic value
  `proceed`). That table's own header already quotes, verbatim, the MUTCD Section 4D.04 sentence
  for green, and that same sentence enumerates FOUR distinct permitted movements — that the
  color-to-meaning schema collapsed into one word: "permitted to proceed straight through or turn
  right or left or make a U-turn movement." New `green_signal_permitted_movement(movement)` table
  (keyless — all four movements are simultaneously true for a green signal, unlike a keyed
  lookup): straight_through, turn_right, turn_left, u_turn. No manifest objective, matching
  `traffic-lights.adj`'s own precedent of not having one. New e2e test file
  `facts_greensignalpermittedmovement_e2e.rs` (3 tests: forward recall with citation, all four
  movements present, full coverage with no abstention). First and only strong candidate from the
  transportation/ sweep tranche — two leads were rejected: red's "unless entering the intersection
  to make another movement permitted by another signal indication" exception clause (too
  thin/note-like, qualifies the already-recorded `stop` value rather than adding an independent
  fact) and yellow's termination-sequence clause (states only one transition, too thin for a
  standalone table). transportation/ (1 table) is now effectively closed. **This completes the
  full 1-2-table-domain sweep initiative** covering geometry/, mathematics/, geography/,
  metrology/, agriculture/, art/, calendar/, environment/, money/, music/, nutrition/, optics/, and
  transportation/.
