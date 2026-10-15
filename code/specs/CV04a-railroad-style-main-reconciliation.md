# CV04a: bounded Railroad style reconciliation

Published bf1ed0f0 is locally accepted, but incoming main7757bb52 (#17537)
introduces style configuration and causes an actual merge conflict. Keep one
implementation PR. Source63 native and source9aa diagram receipts remain frozen;
Miri stays DEFERRED/PENDING, never passed. Do not query or mutate its checkout,
inputs, target, process or receipts. Use the existing separate delivery target.

Commit this specification before merging or changing tests/production. Capture
complete base/owned/incoming inventories, import the producer change, and prove
all unrelated objects and pure incoming paths retained. Resolve genuine textual
overlaps explicitly, preserving both the bounded spacing implementation and the
new producer style fields, defaults, metadata, font-based geometry and tests.
Record conflict resolution separately from independently automatic merge output.

Incoming font_size reaches floating clamp bounds before the owned derived-layout
check; NaN or negative caller-constructed font sizes can panic the checked API.
Huge finite font sizes, stroke widths and marker radii also lack a supported
numeric boundary. Extend scalar admission before any layout arithmetic: all six
numeric fields are finite and <=65536; spacing, strokeWidth and markerRadius are
nonnegative, while fontSize is strictly positive. Do not clamp accepted input.
Preserve the old spacing errors/API; add a precise style-field typed refusal.
The infallible compatibility wrapper remains documented as requiring supported
IR. Parser fontSize zero keeps a positive-number error. All dialects, init
directives and YAML front matter must have identical numeric admission.

Preserve new typography and portable colors, marker visibility/radius, strokes,
fills and metadata. Font-based producer defaults intentionally change leaf
height from16 to16.1 and minimum width44 to44.1. Adapt owned first-principles
numeric and complete glyph oracles explicitly to that formula, retaining their
matrix coverage and complete ordered connector/leaf/glyph comparisons. Add all
seven style metadata keys to the previous exact three-key oracle. Keep the
existing enclosed label box. Verify explicit nondefault style survives complete
typed config, ordered paint geometry/styles and glyph traces, including visible
and hidden markers, rather than checking that a token merely exists.

Tests precede repairs. Preserve genuine red execution and compile failures as
different evidence. Test NaN, infinities, negative, cap+1,1e20,1e308 on each new
numeric field through all four dialects and both source forms, plus fontSize0.
Test typed checked refusal without panic for each bad style with empty/nonempty
rules and title states. Verify exact accepted scalar values, positive tiny,
default, fractional and cap fonts, and zero/default/fractional/cap stroke and
radius. The older168 parser/84typed spacing refusals, complete geometry/glyph
controls, all hop32/16,8192,geometry16128 and production limits remain active.

This bounded numeric reconciliation does not claim arbitrary font/long-label
fitting, marker/stroke visual enclosure, forged LayoutedIR paint admission,
recursive depth/remeasure bounds or complete Mermaid typography parity. Keep
those debts explicit; do not silently count them as accepted safety contracts.

The incoming default-metric counterexample at fontSize64 additionally needs a
font-scaled declared label box: use max(24,fontSize*1.15) capped by the leaf's
height, preserving normal/default sizing. Give rule names max(28,fontSize*1.15)
height centered at the existing rule center. Test the complete ordered glyph
trace and real 0.8/0.2-em metric bounds at tiny, default, fractional,64 and cap
sizes. This closes the concrete fixed-height regression without promising
arbitrary custom-font or long-label fitting.

Run complete affected fourteen-package diagram/parser/paint/downstream suites
and strict lint on fixed source. Prove the full native/compiler dependency
inputs unchanged from accepted63 or rerun affected native suites. Registry
identities/checksums remain exact; no new crates.io dependencies. Update package
READMEs/changelog shards, backlog/issues, documentation checks, build plan and
independent exact-head security review before pushing to the same PR. Current
exact-head terminal acceptable CI and conflict-free/review-clean state gate
auto-merge, then verify actual fetched-main merge reachability before cleanup.

Full Closure parity, output-byte/source joins, stable identities and every
fold/rename/inline/motion/merge/delete lineage requirement remain OPEN, along
with CV04b. Deliver implementation first and genuinely complete final-source
Miri later; do not disable the heartbeat or claim full completion meanwhile.
