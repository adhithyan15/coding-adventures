# CV04a two-point spline integration

## Source frame and bounded scope

Published independently reviewed Closure/shared head is
f4b594d71d238eccc142cd71c39fe096b408de9b; actual signed-root source4f/c10
passed1253/120 with strict4/all11production/44native. Portuguese3774 is a
separately reviewed source-equivalence supplement, not new native execution.
Latest main e7c6bc81064bc80c9a908e102b76cdb1977fe768 adds Cardinal,
CatmullRom and Natural Swimlane curve modes (#17376). Its two-point routes
reuse the existing Monotone collinear cubic controls; this is not a general
multi-point spline solver. TaskApp49096 (#17377) changes only release docs.

Commit this specification before merging main and adding controls. Preserve
exact upstream semantics, all original controls, marker-reference order,
zero-length behavior, resource limits and dependencies. Do not change native
implementation absent an independently demonstrated regression needing its
own bounded repair specification.

## Exact authored controls

Extend the existing authored parser/layout/paint marker helper to the three
canonical spline names in both LR and TB. Require the unchanged42-unit endpoint
geometry, controls exactly1/3 and2/3 along the route, complete cubic commands,
canonical scene metadata, both visible marker triangles with exact vertices,
fill/stroke and area50. Retain all earlier Bump/Step/Basis/Linear/Monotone
assertions and zero-route control without relaxing them. Add a parser alias
control for catmullRom/catmullrom/catmull-rom, all mapping to the same semantic
variant. Keep the new upstream corpus and existing compatibility assertions.

## Validation and retention

Derive the actual reverse dependency cone from changed diagram crates; include
Arc2d and its native consumers so retained raw signed/rotated/zero tangent and
neutral-oracle controls stay exercised. Run the full affected suites, strict
all-target Clippy denying warnings/unknown-lints, original exact authored and
parser work controls, neutral geometry oracle/Python/Ruff, docs/lessons/whitespace.
Freeze explicit integrated main/source/target in receipts; separately record
newer live refs. Require actual process exit zero before source edits or other
checkpoint writers. Preserve failed/partial receipts rather than relabeling them.

Complete compiler/LANG/core/Closure/program/CI/script objects remain identical
to publishedf4 except the disclosed grammar corpus and TaskApp README/changelog.
Retain compiler1253/120 at actual4f/c10 via full objects, with no duplicate
production matrix run. Retain SwiftUI525 six-package637/30/strict6/Python46/
fresh-profile1 via complete objects; TaskApp runtime/config and its scripts must
match while its two release documents are disclosed. Portuguese content/TS and
TaskApp release/version-publication claims remain outside native acceptance.

Final backlog-only complete object/payload proof and independent exact-final
security precede guarded publication to the SAME sole PR17288. Keep currentf4
CI running until a reviewed replacement workflow exists. Fresh main/open-PR
inventories, current conflict-free/review-clean exact-head terminal acceptable
checks plus whole three-platform success and real Closurec BUILT rows gate
auto-merge. Actual fetched-main reachability/payload proof precede cleanup/CV04b.

Full final-source Miri remains **DEFERRED/PENDING**, never passed. Frozen run,
497inputs/checkout/receipts/target are untouched without polling, termination
or duplicate execution. Deliver implementation first, then genuine final-source
Miri with invalidated-input reruns. BaseESTree/owned types/no newcrates.io deps.
CV04b/full Closure parity/output joins/all transformation lineage/full goal
remain open; heartbeat active.
