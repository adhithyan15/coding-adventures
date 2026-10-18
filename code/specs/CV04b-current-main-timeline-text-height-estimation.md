# CV04b — Timeline scalar text-height estimation

This spec follows carrier `7f9e541c850b7a874d8c143cb9caa2509c2d352c`.
The complete 46-path endpoint inventory against main
`1ff5c89aa3773f96990dfa0db4395dae700ce24d` is preparatory evidence:
`0cda3f7f8471703efc9e19801f05db025ca8d82a77195b2e6b00efe34d2f2f94`.
It does not accept incoming main or resolve the remaining merge conflicts.

Adopt the incoming scalar-count height heuristic for period cards and separate
events, with checked represented arithmetic and explicit cumulative event
advances. This estimates vertical space; it does not shape text, change the
authored string, insert line breaks, prove backend fit, or promise containment.
The painter's fixed period-label height22 remains a separate limitation.
Title and section minima, owned widths (TD event minimum160), grouped LR event
origin, palette ordinals, item order, metadata and font roles remain owned.

Miri is DEFERRED/PENDING, never passed. Preserve its frozen run, checkout,
inputs, target and receipts untouched and unpolled. Use a new isolated Cargo
target for this slice. Full Closure parity, source/output joins, all requested
transformation lineage, whole-main reconciliation, hosted checks, actual merge
and genuine final-source Miri acceptance remain OPEN.

## Admission and estimator

Retain raw error order Canvas, Padding, LeftMargin, FontSize, then the four
eager planned role products from the size-minima spec. Admit those products
before allocation and section predicate/visit work, even for empty diagrams.
Use the existing TimelineGeometry error for new derived arithmetic refusal.
No new magnitude, string, node, line or count cap is permitted.

For each actual period or event label, let w be its owned box width, s the
admitted positive finite configured size, and m its admitted role minimum:

1. Capacity is `floor((w - 20) / (s * 0.44)).max(1)` converted to usize with
   Rust's defined saturating float-to-integer conversion. Certify w and w-20
   as finite and positive. Certify s*0.44 as finite; zero from subnormal
   underflow is allowed and gives positive infinity in the quotient, hence
   usize::MAX capacity. An infinite positive quotient also saturates. This
   capacity conversion is an explicit scalar-count heuristic, not admission
   of an infinite layout coordinate or a new resource cap.
2. Iterate Rust `str::lines()` (including its CRLF and trailing terminator
   semantics). For each returned line count Unicode scalar values, replace
   zero with one, then use integer div_ceil(capacity). Sum with checked usize
   addition. Replace an empty sum by one. Spaces, combining marks and wide
   Unicode characters receive one count per scalar; no grapheme or glyph
   width inference is made. Do not allocate a wrapped copy of the string.
3. Evaluate `(lines as f64 * s) * 1.2 + 12` in that order, checking each
   multiplication and addition as finite before applying max(m). A failed
   operation returns TimelineGeometry. The f64 conversion of the actual
   usize count uses Rust's normal rounding; there is no count truncation.

Consequently an empty or short event at size16 has estimated height31.2,
above its old minimum30; short default period cards remain48. At size24 a
short event remains45. Long labels and explicit hard lines may grow both
roles. At tiny positive sizes the existing literal minima remain effective.

## Cumulative vertical placement

For TD, compute each period height using width180 and its role minimum.
Keep its y, event x and event width calculation. Begin event_cursor at y.
For each event, compute its individual height at the actual event width,
emit the existing link then event at event_cursor with that height, and
advance `event_cursor + (event_height + 8)`, checking the inner and outer
additions. Link centers use the individual card/event half-heights. After
all events compute `max(0, (event_cursor - y) - 8)`, checking subtraction
operations before max, and compare with the card height for the row height.
No events retain card height. Preserve the existing row+18, section+10,
spine placement, title association and final dimension certification.

For LR, compute each card height using width190. Retain period y=card_y+24.
Begin event_start and event_cursor at `card_y + (card_height + 42)` with
checked inner and outer additions. Emit each event and link in existing order
and advance the cursor as in TD. For nonempty event lists compare row_height
with `(card_height + 24) + (event_cursor - event_start)`, checking each
represented operation. This explicit grouping retains the owned grouped
origin and avoids subtracting a large origin after adding it to the row
height; it deliberately does not adopt incoming main's left-associated
`card_height + 24 + event_cursor - event_start` expression. Empty lists still
contribute the individual card height. Other LR anchors, widths, section
advances and final frame/rounded-arrow certification remain unchanged.

The estimator admits derived values, while the existing final validator
continues to certify actual represented rectangles and arrow triangles.
Do not weaken that validator, panic compatibility API or error priorities.

## Test-first acceptance

Commit this spec before tests and production changes. Keep all old authored
source and IR fixtures, literal expected layouts/Scenes/query products, raw
error-priority and work-counter controls. Identify every existing expected
product affected by height31.2 and cumulative placement. Retain its entire
old literal, verify that old shape, then declare a complete new expected
product with only specified geometry changes. Never normalize actual outputs
or silently rewrite old assertions. Unaffected products remain byte-exact.

Add independent complete TD and LR layouts with two periods, unequal event
heights, long labels, hard lines, CRLF, trailing terminators, empty strings,
spaces, scalar versus grapheme controls, named/unnamed sections and empty
rows. Include exact scalar-capacity boundaries, adjacent f64 arithmetic
refusals, subnormal admission, error precedence and eager zero-section-work
refusals. Compare complete source/IR/layout/Scene/font-query products for
representative wrapping fixtures without changing their authored strings.

Run compiled unfiltered semantic red before production edits, saving actual
exits and executable identities separately from runner acceptance. Recompute
the affected dependency cone and target census; run its native/downstream
suites, doctests and strict all-target checks using the new isolated target.
Obtain independent exact-source and actual-terminal security review. Update
README/changelog, backlog and owned issue checkpoints before a normal reviewed
progress push to the sole active PR. Any new main remains unaccepted until
separately inventoried/reconciled. Auto-merge requires current conflict-free,
review-clean exact-head terminal acceptable required checks and verified
actual main reachability; no merge approval is implied by this spec.
