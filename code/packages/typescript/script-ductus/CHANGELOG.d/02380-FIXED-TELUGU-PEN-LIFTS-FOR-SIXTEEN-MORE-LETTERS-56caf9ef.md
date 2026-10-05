### Fixed Telugu pen lifts for sixteen more letters

- `త`, `న`, `ప`, `య`, `ర`, `ల`, `వ`, `శ`, `ష`, `హ`, `ఠ`, `జ`, `చ`, `అ`, `ఎ`
  and `ఒ` in `src/strokes/telugu.ts` lifted the pen after numbered or
  ordered movements of their tracing source: 1, 2, 3, 3, 1, 1, 2, 2, 3, 3,
  2, 3, 1, 1, 1 and 2 times. HP Labs India's native-writer samples
  (hpl-telugu-iso-char, counted from the prototypes in the MIT-licensed
  LipiTk 4.0 Telugu recognizer) put the modal stroke count at 1, 1, 2, 3,
  1, 1, 1, 1, 2, 2, 2, 2, 1, 1, 1 and 1.
- Fifteen of them now have that modal count; in the same order, skipping
  `ష`, that is 0, 0, 1, 2, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0 and 0 lifts. `ష`
  drops to 2: its tail leaves the right body partway up and the source
  draws it after the body, so joining it would reverse the order or retrace
  the body. The movements, their order and their directions stay, as
  segments inside the merged runs. Lifts fall only where a part stands
  apart or the pen sits at a dead end: `ప`'s separate flourish, `హ`'s
  separate chevron, `ఠ`'s inner dot, the tip of `జ`'s lower-right bowl, and
  `య`'s angled join and right bowl.
- Where Noto Sans Telugu joins the flourish or chevron to the body (`త`,
  `న`, `ర`, `వ`, `శ`, `ఠ`, `చ`), the run climbs up its left arm before
  drawing it; that is a new segment. `ఎ`'s arch, `అ`'s right lobe and `ఒ`'s
  left bowl now continue from where the previous movement ends.
- Every centre line of the merged runs was refitted to the bundled outline.
  `fractionOnInk` is 1.0000 on every stroke except the unchanged flourish
  of `ప` (0.9912) and chevron of `ష` (0.9894), so the `అ` (0.96), `త`
  (0.83) and `ఒ` (0.84) on-ink overrides and the `త` and `హ` (0.05)
  untraced overrides are removed. Only `ష` leaves ink untraced, the tip of
  its unchanged chevron (0.7%). Every join inside a run is exact.
- `ఆ`, `ఇ`, `ఏ` and `ఝ` keep their lifts: in Noto's outline, joining
  their parts would skip ink the source draws (`ఆ`), reverse a movement
  (`ఇ`, `ఏ`), or retrace half of an open bowl (`ఝ`).
- The sixteen tests now pin the grouped runs with every segment label in
  order, and note the native share. `tests/ductusview/telugu.test.ts` now
  pins `అ`'s filmstrip as one unbroken run of four movements.
- `tests/stroke-ownership.test.ts`: only `nonTamilDataHash` moves. The
  filmstrip-geometry ledger was regenerated.
