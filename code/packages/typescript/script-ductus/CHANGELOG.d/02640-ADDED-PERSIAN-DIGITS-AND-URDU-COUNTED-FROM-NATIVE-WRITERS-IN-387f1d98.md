### Added — Persian digits ۰-۹ and Urdu ۰-۳, counted from native writers in POH-Db

- **Why.** The Persian digit lessons (FA-W19, four lists) and the first two
  Urdu digit lessons (UR-W31 zero-one, two-three) printed no strip: no digit
  had a cited stroke order.
- **Source.** POH-Db, the Persian Online Handwriting Database (SLT Lab,
  Amirkabir University of Technology; `SLTLabAUT/POH-Db` at commit
  `018b039`), NumberGroup writepads: native writers' handwriting recorded as
  InkML, one trace per pen-down stroke. AGPL-3.0, so the records cite counts
  and shares only; no coordinate or trace is copied. Counted for these
  records from 112 writepads by 30 writers (mouse input left out); of the
  1,046 lines whose character count matched their label, the 1,005 written
  left to right were used.
- **What the writers do.** Every digit is one stroke (99-100% of 447-676
  samples each), so every `penLifts` is 0. ۰ is a dot with no settled start
  or direction (53/47); ۱ runs down from the top left (85% start there); ۲ and
  ۳ start top right (96%, 97%), cross the top to the stem's head at the top
  left (97%, 96%) and come down the stem; ۴ curls over from the top right,
  goes out along its lower arm and back (71%), then into the head and down;
  ۵ goes down its right side first (95%) and closes at the top; ۶ curves down
  the left, swings up to the right (95%) and slants down to the lower left; ۷
  is down then up (top right 93%); ۸ up then down (bottom right 95%); ۹ draws
  its loop leftward first (99%) and the stem last (97%).
- **Paths.** `src/strokes/arabic-family.ts` holds one table of digit strokes,
  fitted by hand to the bundled Noto Naskh Arabic outline at the default
  tolerances (on ink 1.0000 on every stroke, nothing untraced beyond 1.1% on
  ۶, no override). ۶ starts at the top centre (29% of writers) so Noto's
  curl, which reaches further right than most writers begin, is still traced.
- **Urdu.** `urdu-nastaliq:۰`-`۳` draw the same paths (fresh copies) and
  cite Urdu rows that say the writers were Persian and the evidence is by
  shared form (medium confidence). Urdu ۴, ۶ and ۷ are different shapes, so
  Urdu ۴-۹ get no ductus.
- **Pins.** `tests/strokes/arabic-family.test.ts` pins every digit's labels,
  one-stroke run, citation and font, and the places its labels name (start
  and end thirds, the stem as the last movement, ۵ and ۹ clockwise, ۰ small
  and anticlockwise, ۴'s trip out and back); `tests/ductusview/
  arabic-family.test.ts` pins "one unbroken stroke · N movements" for all
  fourteen. Perso-Arabic 24 -> 34 and Urdu-Nastaliq 31 -> 35 ductus keys:
  stroke-ownership pins and `_registry.json` script runs regenerated; the
  filmstrip-geometry ledger gains 14 entries.
