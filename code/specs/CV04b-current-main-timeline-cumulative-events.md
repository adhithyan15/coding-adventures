# CV04b — checked cumulative Timeline event positions

This bounded slice follows published carrier
`7071e255e1be947f9a33b65115ab5ff8758f91ef`. Re-audit the sole open shared-crate
PR and origin/main before publication. The retained 48-path endpoint inventory
against main `1ff5c89aa3773f96990dfa0db4395dae700ce24d` has SHA256
`f068a86f408b49078b5faa890d7ec4f545e572696e291d2d574f1826f93c4d94`.
The latest audit observes eleven virtual conflict paths; incoming main remains
UNREVIEWED/UNINTEGRATED. This contract does not accept that main wholesale.

The next-item description combined event estimates with cumulative positions.
Separate these changes before tests or production edits: establish checked
cumulative positions first, keeping existing event height/pitch. Event scalar
height estimation would change default-height30 products to31.2 and requires
a later explicit complete-product migration. This slice therefore changes the
represented addition order without introducing that independent height change.

## Represented operations

Keep raw admission Canvas, Padding, LeftMargin, FontSize and the four eager
finite role products, including the event minimum and pitch. Keep the existing
period scalar estimator, widths (TD event minimum160), palette ordinals,
section predicate/visit counts, metadata, original strings/IR, and all actual
rectangle and rounded-arrow checks. No new public API/type/dependency or cap.

For TD, initialize an absolute event cursor to the period's y. For every event,
use that cursor directly for event y and add half the current height for its
link endpoint; after appending the pair, evaluate and finite-check
`cursor + event_pitch`, including the final event's advance. Calculate the
event span as finite `cursor - period_y`, then finite `span - 8`, clamped to
zero, and take max with the period height before the existing row advance.
An empty event vector has span0 and makes no cursor addition.

For LR, keep the initial grouped origin exactly
`card_y + (card_height + 42)`, but finite-check that value before the event
loop. Preserve it as event_start_y. Use the absolute cursor for event y and
the link endpoint. After each event, finite-check `cursor + event_pitch`.
Only a nonempty event vector contributes to the row maximum: calculate the
finite span `cursor - event_start_y` and add it to the finite grouped
`card_height + 24`. Retain the existing final grouped row/section advances.
Do not rewrite this as `(card_height + 24 + cursor) - event_start_y`.

All arithmetic refusals use TimelineGeometry. Finite cursor arithmetic is not
proof of actual rectangle or arrow validity; keep the unchanged final checks.
The cumulative cursor is the intended observable f64 association, not a
mathematically equivalent indexed multiplication or a normalized output.

## Test-first acceptance

Commit this spec before tests or production changes. Retain all old authored
sources, semantic IR, complete literal layout/Scene/FontQuery controls, error
and work-count tests unchanged initially. If an old complete product changes,
inventory its complete old/new shapes and review the migration explicitly;
never update expected values from actual layout output. In particular retain
large-coordinate arrow and grouping controls as independent refusal evidence.

Add complete literal TD/LR layouts at default16 and fractional16.01, each with
four cumulative events, a following no-event period, an empty unnamed row and
a named empty row. Verify every item, cursor-dependent link endpoint, row and
section advance, palette ordinal, spine and metadata. Fractional arithmetic
must distinguish cumulative y from indexed multiplication. Use declared
literal floating-point results derived independently from the layout. Keep
ordinary default products exact. Add empty and invalid raw/eager admission
controls where they extend the new contract rather than duplicate old tests.

Run compiled unfiltered semantic red before production changes; a missing
symbol or compiler failure is not semantic red. Recompute the affected
manifest graph and target census. Use a fresh isolated Cargo target, retain
actual exits/raw/harness and executable identities, and run both temporal and
painter native suites, doctests, strict all-target checks and documentation
checks. Independent exact-head security review precedes progress publication
to the sole PR. Old tests that require a migration must stop this slice for
explicit spec/oracle review rather than silently relax safety expectations.

Miri is DEFERRED/PENDING, never passed. Preserve the frozen run, checkout,
inputs, receipts and target untouched and unpolled. Full Closure parity,
stable node identities/spans, output-byte/source joins and full fold/rename/
inline/motion/merge/delete/pass-sweep lineage remain OPEN. Whole-main conflict
reconciliation, required exact-head hosted checks/review, actual merge and
genuine final-source Miri remain required. This dependency slice does not
certify shaping, glyph containment, backend/raster fit or full completion.

## Bounded execution checkpoint

Timeline events now use an absolute cursor and checked addition of the existing
finite event pitch after every event, including the final advance. TD row height
uses the represented cursor span minus8, clamped to zero; LR retains its grouped
initial origin and adds the represented span to grouped card_height+24. This
replaces indexed multiplication, so some fractional or large-coordinate products
change by represented f64 bits. Event minima/heights/pitch, period estimates,
widths, palette ordinals, authored strings/IR, metadata/fonts, raw/eager error
priorities and actual rectangle/rounded-arrow guards remain. No public API,
dependency, arbitrary cap, shaping or glyph-fit guarantee is introduced.

Native source `59a60792c174b0a601c8d9f6f278d14f113fd817` passed292tests (102temporal+190painter, Windows e2e0),
with0failures/ignores/filters across3unfiltered harnesses. Two doctest invocations
contain0tests;2strict all-target checks and metadata passed:8actual commands0.
All466old completion identities and6new complete typed products pass. Compiled
semantic red at0b01e80e238b4def271624942b4b5e5c97aceec7 was289P3F, not a compiler
failure. Both baseline and new whole-product controls remain, including p54
rounded-span, p55 accepted and p56/57 arrow refusals; LRpadding0 and TDfractional
witnesses separately distinguish cursor coordinates from indexed multiplication.
The earlier runner durability gap, missing LR discriminator and anticipated
literal correction were resolved before execution and remain in filled lessons.
Green receipt SHA256 `117b380da3fa54314337fec1ae351d611fad86518da97e9f73c031d2833f1d31`; immutable EXE/raw manifest `74f958ed4ab1c83cd64306a217bfed183895622d8dd1e439fe043838fdae18c3`.
Exact-source and actual-terminal independent reviews PASS/RELEASED. Native targets
are RELEASED_DO_NOT_REREAD; this Markdown carrier does not claim a Rust rerun.

Event scalar-height estimation/default30-to31.2 product migration is next and
remains separately unimplemented. Whole-main/conflict reconciliation, required
exact-head hosted checks/review and actual merge remain OPEN; sole PR17288 is
unmerged and auto-mergeOFF. Miri DEFERRED/PENDING, never passed; frozen run,
checkout, inputs, receipts and target untouched and unpolled. Full Closure
parity/stable identities/spans/output-byte-source joins/all fold-rename-inline-
motion-merge-delete/pass-sweep lineage remain OPEN. Genuine final-source Miri
follows implementation delivery. No compiler-wide or platform-renderer execution
is claimed. Older sections are historical evidence for their named source heads.
