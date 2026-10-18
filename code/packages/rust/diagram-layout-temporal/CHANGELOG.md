# Changelog

## Unreleased — checked Timeline size-driven minima

Timeline title/section/period/event heights now use positive finite size s
with minima max(2.5s,40), max(1.875s,30), max(3s,48), max(1.875s,30).
Every planned product is checked before max and section work, including empty
or untitled input; overflow returns existing TimelineGeometry. Raw Canvas,
Padding, LeftMargin and FontSize priorities and full represented-frame/arrow
certification remain. Ingress/IR still transports every positive finite size.
Indexed event pitch and dependent vertical advances retain the specified f64
association, with an accepted bit-exact default-title regression witness.
Widths, grouping, palette ordinals, authored strings/IR, font roles, metadata,
ordered font identities and dependencies retain their existing contracts.
Wrapping, containment/nonoverlap and arbitrary backend/raster guarantees are
separate work; fixed period text height22 and LR row anchors remain.

Executed source `ed6526c30e3c1f35b79dbae244a069eea35c31b6`: 283 native tests passed, zero failures,
ignores or filters, across three unfiltered harness invocations (Windows e2e
contains zero tests). Two doctest invocations contain 0 tests; two strict
all-target lint checks and metadata passed: eight actual commands0. Fresh
1293manifest/1292node/436registry graph gives reverse2/forward50. No new
compiler-wide or Apple rendering execution is claimed. All177 old temporal
cells (including104 typed size cells) and210 old painter cells are retained;
17 new complete typed cells plus3 whole source/IR/layout/Scene/query products
pass. Original literal bodies are retained; old MAX Results and12 custom
root24 products have explicit closed expected migrations. Actual compiled
semantic red at `e4d9e38d3ec6741b6de7017cfd9fe44de8c4c14c` was275P8F, before production.
Both red executables and all raw streams are retained separately. Five filled
lessons record preparatory static risks/refusals, positive-verdict guard repairs
and a consumed mutation-session ordering mistake; none is mislabeled a compiled semantic red or passed test.
Exact source and actual-terminal independent security reviews PASS/RELEASED.

PR17288 remains unmerged. Whole-main/conflicts, current review-clean exact-head
terminal acceptable required CI and actual merge reachability remain open.
Miri DEFERRED/PENDING, never passed; frozen run/checkout/inputs/receipts/target
are untouched and unpolled. Full Closure parity, stable identities/source spans,
output-byte/source joins and fold/rename/inline/motion/merge/delete/pass-sweep
lineage remain OPEN. Genuine final-source Miri follows implementation delivery.
Green receipt SHA256 `8a1ae70875d93cb07b0a8c5117a90809b110f9e4d1fbda29e1a03f2e092a0ecf`; immutable manifest `1ef0fb729dee9fb76a592540afabf534eab17bdfac70dbcbb12da256272173cb`.

## Historical — Timeline size ingress and checked admission

TimelineConfig now carries font_size with a16.0 default. Only direct fontSize
in the first closed init root, with absent-member fallback to config.fontSize
YAML, supplies the value. Existing decoding/unit scales and ownership remain;
invalid source values fall back to16.0. Checked Timeline layout rejects invalid
typed sizes after the established canvas/padding/margin priorities, using the
new appended TimelineFontSize error. Every positive finite size, including
subnormal/MAX, is transported without changing the existing layout formulas.
This is ingress/admission only. Caller paint sizes, glyphs, metadata, font roles,
queries and current family transport remain exact; size-driven typography and
wrapping still require their own bounded contract and represented-operation tests.
At `a3172c6569863bad3c02075f3090443c94f6c8cb`, 1181 native tests passed across59 unfiltered harnesses
(0 ignored); 2 doctests passed/0 ignored across14
libraries; all14 strict all-target checks passed. New complete controls cover
132source IR cells,84positive/empty typed layouts,12invalid inputs,8priority
cases and compatibility panic, retaining all old complete Scene/query controls.
Public API migration:56 mandatory literal fields+2field/default declarations
and1appended error variant, with five true struct updates unchanged. The retained
custom root24 source changes only12 expected semantic/layout config values.
PR17288 remains unmerged with current-main/conflict/review/hosted/actual-merge
gates outstanding. Miri DEFERRED/PENDING, never passed; full Closure parity,
transformation lineage and output-byte/source mapping remain OPEN.


## Unreleased — checked separate timeline events

- Admit dedicated Event and EventLink items through actual outer/text frames
  and the painter's represented arrow determinant; preserve all old guards.
- dc842d0c passes258 affected tests, two empty doctests and both strict checks;
  keep all48 old typed palette Scenes and add32 complete source Scenes plus
  four legacy embedded-event Scenes. No Apple rendering claim.
- Historical835f9767 passed258 tests but failed painter strict on some_filter;
  its aggregate remains false. Miri DEFERRED/PENDING; full Closure goal OPEN.



## Unreleased — checked timeline geometry

- Add explicit checked numeric errors, represented outer/text frame validation,
  real-period TD spines and one-time section detection; preserve other dialects.
- Genuine compiled red4P25F precedes22638c8b full76P/strict0; source-derived
  Scene/palette/downstream validation remains pending. Miri DEFERRED/PENDING.

## 0.36.0

- Resolve bounded three-digit Gantt years into backend-neutral temporal geometry.

## 0.35.0

- Resolve typed signed variable-width Gantt years before backend-neutral layout.

## 0.34.0

- Resolve typed Gantt quarter dates to their first month before backend-neutral layout.

## 0.33.0

- Resolve typed unpadded Gantt time fields into backend-neutral temporal geometry.

## 0.32.0

- Validate typed numeric and two-letter Gantt weekdays before backend-neutral layout.

## 0.31.0

- Resolve and validate typed Gantt ordinal days before backend-neutral temporal layout.

## 0.30.0

- Resolve typed one-, two-, and three-digit Gantt fractional seconds into precise backend-neutral geometry.

## 0.29.0

- Resolve and validate typed English weekday names before backend-neutral Gantt layout.

## 0.28.0

- Resolve typed 12-hour Gantt clocks into backend-neutral temporal geometry.

## 0.27.0

- Resolve single-component Gantt second timestamps into precise backend-neutral task geometry.

## 0.26.0

- Resolve typed Gantt timezone offsets into UTC-normalized backend-neutral task geometry.

## 0.25.0

- Reserve backend-neutral row geometry for multiline Mermaid Gantt titles, sections, and task labels.

## 0.24.0

- Resolve typed sub-day Gantt durations at millisecond precision before backend-neutral temporal layout.

## 0.23.0

- Resolve typed Gantt calendar, textual-month, time-of-day, and Unix timestamp input formats into backend-neutral temporal geometry.

## 0.22.0

- Preserve combined Gantt task tags through layout and lower `vert` tasks to full-height markers without consuming task rows.

## 0.21.0

- Resolve chained and multi-source Gantt starts to the latest dependency end and `until` ranges to the earliest referenced start.

## 0.20.0

- Apply `inclusiveEndDates` to explicit Gantt end dates and place the standard bottom axis plus optional top axis.
- Resolve the current-day marker and supported inline stroke declarations before backend-neutral Paint lowering.

## 0.19.0

- Extend Gantt task bars and dependency starts across excluded calendar days, honoring explicit includes and weekend boundaries.
- Resolve configured Gantt axis formats and tick intervals into backend-neutral time-axis labels.

## 0.16.0

- Route GitGraph merge and cherry-pick arcs from resolved source commits instead of event-order guesses.

## 0.15.0

- Lay out GitGraph lanes, commits, and merge arcs in LR, TB, and BT directions.

## 0.14.0

- Resolve GitGraph commit types and event kinds into backend-neutral commit symbols.

## 0.13.0

- Resolve GitGraph branch lanes with Mermaid 11.16.1 explicit and implicit ordering semantics.
- Emit branch labels in deterministic lane order for backend-neutral paint lowering.

## 0.12.0

- Preserve ordered GitGraph tag lists on temporal commit nodes.

## 0.11.0

- Reserve a backend-neutral GitGraph title row and carry accessibility metadata into temporal layout IR.

## 0.10.0

- Lay out Journey sections and tasks horizontally with score-ranked faces and activity lines.

## 0.9.0

- Resolve Journey task offsets and deterministically wrap actor legends to configured bounds.

## 0.8.0

- Resolve cyclic Journey actor, section fill, and section text palettes.

## 0.7.0

- Resolve styled Journey titles into dedicated backend-neutral layout items.

## 0.6.0

- Resolve Journey task typography onto backend-neutral temporal layout items.

## 0.5.0

- Resolve Journey margins, task dimensions, and task spacing from Mermaid init configuration.

## 0.4.0

- Resolve sorted Journey actor legends and deterministic actor colors.

## 0.3.0

- Reserve dynamic Journey row heights for normalized multiline labels.

## 0.2.0

- Lay out Mermaid user-journey sections and scored task rows.

## 0.1.0 — 2026-04-24

### Added
- Initial release as part of DG04 extended diagram families
- `layout_temporal_diagram(diagram, cw)` — lay out Gantt or git-graph
- Gantt: two-pass date resolution (absolute dates + `after <id>` deps)
- Gantt: weekly axis ticks, section headers, task bars, milestone markers
- Git-graph: branch lane assignment, commit nodes, merge arcs
- 7 unit tests
