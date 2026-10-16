## Stable Wardley annotations and total box placement

Annotation groups retain first authored number order, every repeated coordinate,
and the first nonempty decoded text. An ordered index replaces repeated vector
scans. Active quotes preserve interior spaces, Unicode and percent-percent;
only the active delimiter and backslash may be escaped. Malformed tokens retain
exact physical source errors. Typed box and marker coordinates are finite in
[0,1], with named refusals after note coordinates and before reference checks;
every derived box and marker point is finite.

Boxes use the existing effective plot edges. Width is the smaller of desired
width and max(plot width,40); height retains all nonempty entries. Clamp upper
bounds are never below their lower bounds. Insufficient space pins the origin
to the inner left/top and permits overflow. No entry is dropped or font reduced.
This defines placement, without a general fontfit or containment guarantee.
Marker700/10, entry caller-weight/11 and note700/11 retain caller family, italic
and stretch. The checked minimum420x300 map and stage expression are unchanged.

Actual exact source1e13fc69 passes parser10, work3, geometry45 and Paint4 focused
tests:20 new complete IR/34 exact errors;48legacy+48named/6priority/24derived
annotation refusals;32 whole source/IR/layout/scene/glyph/all18metadata/ordered
cached-query cells, comprising16 old and16 new. New literal scene oracles were
committed before rendering. The real default30-entry clamp48..44 panic was
reproduced; its failed wrapper word check remains separate from actual Cargo101.
All new cells passed on first repaired render without oracle corrections. Old
scene migrations add only required empty annotation ABI and zero count metadata;
all numeric/font/query/color/stroke assertions reverse byte-exactly.

Full affected14 packages pass1000P0F0I across53 target identities; strict all-
target warnings/unknown-lints denial exits0. Tests4b007728/strict86be17a7 bind
style14reverse/50forward/844objects, raw1476 Rust/TOML, native2856 exact93bfe3d2,
registry376 and21+3 static resources. Independent source and terminal reviews
PASS. Retained13 Venn fixture files contain232 complete cells;37 Ishikawa cells
and67 prior parser IR controls remain exact. Indexed comparison bounds are
cfg(test) observers, without a production work counter/cap.

This is unmerged progress in solePR17288. The composed source integrates main
5b42502d; later native16 main459274e4 is pending bounded reconciliation, without
current-main validation acceptance. Required exact-head hostedCI, conflicts,
reviews, actual merge reachability and macOS phase-refusal rootcause remain OPEN.
Historical Closure36/3705+3 retains source316e attribution, without fresh execution.
Base ESTree canonical serialized boundary/own types/no new crates.io dependencies
remain owner decisions. Full Closure parity/output-byte-source/full folding,
renaming,inlining,motion,merging,deletion lineage/stableIDs/spans/pass-sweep/CV04b
remain OPEN. Miri **DEFERRED/PENDING**, never passed; frozen checkout/run/inputs/
target/receipts untouched and unpolled, no duplicate or termination. Delivery
first, genuine final-source Miri afterward; heartbeat remains ACTIVE.
