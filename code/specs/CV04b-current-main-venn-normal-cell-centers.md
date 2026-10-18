# CV04b: normal Venn text at checked shared grid centers

Spec-first bounded reconciliation after published CLOC41 carrier
`fbb2bd93fc935025548fc21decbef55785c77a6a`. This selects work; it does not
accept incoming main, execute a test, or resolve the whole PR conflict set.
Fresh 2026-10-18T04:30:44Z audit fetched main
`e69aaf0b972c5d74e91673f28b30b7037896edea`, inspected all14 open PR file lists,
both Closure backlogs and epic15830. Sole Closure/shared PR17288 remains
conflicting, auto-merge OFF, without hosted checks. Rediscover these facts
before publication; the hashes here identify examined inputs, not active heads.

Preparatory source audit `138812e9` / checks `c181e4d7` identifies the normal
attached-text change and the unsafe raw helper. Recheck its complete endpoint
bodies against current source/main before implementation. The owned layout
places every attached text at its membership centroid plus20 in y; main groups
texts by exact authored membership and places each at its grid-cell center.
Main's vector searches and NUL-delimited debug identity must not be imported.
The owned checked entry point, structural debug grouping and numeric admission
remain authoritative. No new crates.io dependencies, production count cap,
manifest-version sweep or raw-main merge is part of this slice.

## Behavior and shared preparation

Retain the complete current canvas, title, circle, set/intersection-label,
style, font and metadata behavior. Replace only attached-text position/order.
Groups use the exact ordered membership vector as structural identity, including
repeated members, reversed vectors and embedded NUL characters. Iterate groups
in first-authored occurrence order, and texts in their authored order within
each group. Thus A0,B1,A2 becomes A0,A2,B1 after the unchanged set/intersection
label prefix. Hash maps index lookups only; they never determine output order.
Keep label-or-ID fallback, text font size and each complete authored style.

Build group membership/counts, first-circle-by-ID lookup and nonempty region
label membership once. Preserve the separately constructed last-center-by-ID
lookup. Duplicate circle IDs therefore still use the first radius/circle for
the area equation and the last center for centroid calculation. Region label
presence is OR over every exact-vector match, including a later nonempty label.
Preserve repeated centroid terms, their reduction order, partially known
memberships, and the empty/entirely-unknown group policy (no text/debug output).

Compute each admitted group's grid once. Normal labels use that grid whether
debug is enabled or disabled. Enabling debug additionally publishes the same
ordered debug circle/cell vectors as before; it must not allocate or compute
an entire second grid. Preserve the existing private debug test entry through
a test-only adapter if needed; retain every original debug assertion unchanged.
All non-Venn functions and existing scalar/derived predicates stay exact.

For count n>0, retain the existing arithmetic sequence: cols=ceil(sqrt(n)),
rows=ceil(n/cols), both at least1. Preserve the exact centroid, inner-radius
minimum, zero-overlap fallback, max minimum box dimensions, label offset and
row-major cell equations from the owned debug implementation. A text position
is cell.x+cell.width/2 and cell.y+cell.height/2, using that operation order.
Do not reassociate floating-point expressions or replace them with rounded
constants. This is a grid reconciliation, not area-proportional Venn parity.

## Independent numeric witnesses before production changes

The existing direct grid fixture has width1920 (scale1.2), center(100,100),
radius20, minimum box96x72. Its complete old debug-cell tables remain intact.
Predeclare these normal text centers independently, with row-major text order:

| Count | Unlabeled centers |
| --- | --- |
| 0 | no labels and no debug vectors |
| 1 | (100,136) |
| 2 | (76,136), (124,136) |
| 3 | (76,82), (124,82), (76,118) |
| 4 | (76,82), (124,82), (76,118), (124,118) |
| 5 | (68,82), (100,82), (132,82), (68,118), (100,118) |

A nonempty exact-vector region label adds5 to every listed y; None/empty do
not. Tests of real try_layout_venn must separately precompute complete expected
circle/layout/normal/debug vectors from authored input. In particular use
width800,height600,padding20, two equal-size4 singleton regions, no title:
available560, orbit computed as560*0.16, radius140 and scale0.5. The orbit is
binary64 89.60000000000001; its rounded display89.6 is not a replacement input.
Retain cos/sin of the actual index*PI angle, including B's sin(PI) term, before
the additions produce centers displayed as(489.6,300)/(310.4,300).
For one labeled A text the expected normal position is(489.6,331), rather
than the old(489.6,320). Before production edits save an independent IEEE-754
operation-order table for counts0..5, label/no label, title/no title and both
debug flags. Use exact values/bits and complete structures, not tolerance or
expected positions recovered from a returned layout. Title variants must
recompute their changed available height and centers independently. With a
title, available526 gives A.x=400+(526*0.16), bits0x407e428f5c28f5c2
(484.15999999999997); literal484.16 is one ULP higher and is not its oracle.
The titled labeled single-A text position is therefore(that x,348).

Add tests using existing entry points first, and consume a genuinely compiled
red. Missing future helper names, setup errors, formatter differences or a
failing old oracle are not the intended regression. Include a passing legacy
debug/control group in that red. Record precise failing assertions and cases;
counts or token presence alone do not establish the cause.

## Work, admission and retained end-to-end controls

Actual test observers must count input/index/group/membership/centroid/radius
and emitted cell/normal/debug visits in both debug modes. Establish a linear
bound on the observed iterator visits and lookup invocations in circles+regions
+texts+region membership terms+all authored text membership terms+group
membership terms+emitted cells/labels. T texts with M repeated members each
contribute T*M authored key terms even if they form one group. Include that
control; do not describe its input as merely T+M. Repeated group members still
count as work. Count real traversals, not extra scans added just for a counter.
The existing
VENN_DEBUG_VISITS witnesses and their limits stay valid; do not raise a ceiling
to cover a rescan. Observer counters are cfg(test), not a production work cap.
Internal Hash/Eq element/byte work, collision behavior, allocation and
whole-process limits are separate from those observer bounds; do not claim
worst-case linear total execution from a HashMap lookup count or item visits.
No per-group full input search.

Preserve Width then Height then Padding then DerivedGeometry error priority,
including empty input, existing maximums, finite negative positions, zero debug
sizes and all old nonfinite-field refusals. The checked result validator must
also reject any newly emitted nonfinite normal position through its existing
label terms. Do not bypass a demonstrated applicable arithmetic failure.

Cover structural NUL collisions; reversed/repeated/unknown memberships;
first-radius/last-center duplicate IDs; later-only nonempty region labels;
interleaved authored groups; count0/1/2/3/4/5/large; title/debug on/off; and exact
whole labels, styles, circles, cells, order and typed error results. Keep all
old debug/config/typography contracts byte-exact unless an independently
reviewed intentional normal-label oracle migration is required.

Exercise source -> complete IR -> checked layout -> whole ordered PaintScene,
glyphs, colors, fonts, metadata and cached font-query sequence at real entry
points. Only the declared normal text positions/order may migrate in existing
whole-scene oracles. Preserve complete before/after inverses and direct forged
layout painter controls; the renderer has no required production change here.
Never generate the expected scene by painting the returned layout. Retain all
four complete old Mermaid Venn corpus programs absent from raw main and their
assertions. Importing unrelated new parser/corpus behavior is outside this slice.
Preserve unconditional portable parser/geometric/hierarchy dev dependencies.

After focused green, derive the current complete all-kind affected graph/cone
and metadata target census. Run appropriate unfiltered native/compiler/consumer
suites, all-target strict checks, independent exact-head source/terminal reviews
and real documentation gates. Attribute each actual execution to its own source;
CLOC41's3277 passes are not execution of this new Venn work. Update README,
changelog, backlog and issue checkpoint before progress publication to the same
sole PR. Keep one active implementation PR and preserve unrelated local edits.

Current-main Timeline/ABI/parser/corpus/native33/34/Qt/CI/attribute/checker
reconciliation and a closed whole-tree conflict recipe remain separate OPEN
work. Full Closure parity, output-byte/source maps and all transformation
lineage remain OPEN. Base ESTree stays canonical, with owned types and no new
crates.io dependencies. Miri DEFERRED/PENDING, never passed; its frozen run,
checkout, inputs, target and receipts remain untouched/unpolled. Implement and
deliver first, then obtain genuine final-source Miri acceptance. No auto-merge
without current conflict-free review-clean terminal acceptable required checks;
verify actual fetched-main merge reachability before cleanup. Heartbeat ACTIVE.
