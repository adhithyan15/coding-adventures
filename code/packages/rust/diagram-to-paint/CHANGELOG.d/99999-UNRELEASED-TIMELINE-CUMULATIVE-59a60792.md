# Checked cumulative Timeline event positions

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
