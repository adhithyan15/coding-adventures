# CV04a: Railroad lexical style sanitization reconciliation

Current main9b3ec98e (#17548) adds authored Railroad color/font sanitization;
intervening d4ff Sanskrit37 paths have no owned/runtime/checker/planner overlap.
Commit this spec before import/tests. Reuse terminal R2 from acceptedc292 on
a new branch; keep R1c0f2 source/HEAD/ordering target frozen during full native
validation49657. One active implementation PR17288, autoOFF. Miri remains
DEFERRED/PENDING, never passed; frozen497 inputs/run/target/receipts untouched.

Frame every incoming path and reconstruct all overlaps. Preserve producer
versions/corpus/spec/Apple-only tests, all owned numeric caps/effective theme
font guard, explicit precedence, exact theme fallback/config matrices and
complete glyph/path/style/metadata oracles. Audit existing helpers before
extending behavior. No production repair is presumed without a failing case.

Adopt the producer's lexical string policy: trim admitted colors; permit
3/4/6/8-digit ASCII hex, nonempty ASCII alphabetic named tokens and the listed
numeric/punctuation function interiors. This is a lexical filter, not complete
CSS grammar or semantic color validation. Font families retain the incoming
ASCII letters/digits/space/quote/comma/dot/hyphen/underscore policy. No arbitrary
font fitting, direct typed-IR sanitization or downstream HTML/CSS guarantee is
inferred. Existing resource/forged-IR debts stay open.

Preserve resolution order: choose theme primary then secondary/default string,
sanitize that selected theme value against field default, then sanitize the
explicit Railroad value against the sanitized theme fallback. An invalid
selected primary theme therefore falls to the field default rather than
retrying the secondary. Explicit accepted values override their theme field;
invalid explicit values use the sanitized theme. Marker fallback uses the
sanitized theme line, independently of an explicit line override. Effective
font numeric0..65536/positive checks and invalid-theme numeric fallback stay.

Add public parser tests with complete expected RailroadConfig for all11 color
fields and font family, every four dialects/init/top/config YAML transport.
Check accepted trimmed values, invalid explicit-to-theme fallback, invalid
theme-to-default behavior, primary-invalid/secondary-valid ordering and marker/
line independence. Retain whole previous matrices rather than changing their
expectations. Add complete portable scene comparisons for sanitized authored
styles: real font queries, ordered glyph/path/shape/marker vectors, styles and
all11 metadata keys. Preserve the old explicit40 and themed40 scene oracles
byte-exact. Apple renderer execution is not inferred from portable success.

Run existing baseline/focused controls, then all14 affected parser/style/paint/
downstream packages plus strict all-target lint on a fixed repaired/imported
head. Native166 canonical Cargo manifest paths/inputs must remain exactc0f2;
no new execution acceptance is inferred from attribution. Preserve376 registry
tuples/checksums and add no crates.io dependencies. Final docs/backlog/issues,
actual checker/lessons/fresh unforced plan, corrected Closure36/3705 historical
compiler1093/181+strict input attribution and independent exact-head security
gate same-PR push. Current conflict-free/review-clean terminal acceptable
required/native hosted checks gate auto; actual fetched-main merge reachability
gates cleanup. Base ESTree/owned types retained. Whole Closure parity/byte-source
joins/stable IDs/pass-sweep/all transformation lineage/CV04b remain OPEN.
Deliver implementation, then genuine final-source Miri; heartbeat ACTIVE.
