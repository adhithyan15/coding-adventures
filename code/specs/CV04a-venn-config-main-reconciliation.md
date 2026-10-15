# CV04a: bounded Venn canvas configuration reconciliation

Commit this spec before integrating main f0ecbfa6 (#17563) into the reviewed
d4004fe5 delivery candidate. Retain its twelve changed paths: Venn semantic and
layout config, new single-argument layout API, parser resolution, five portable
metadata entries, incoming corpus and tests, package versions and partial DG04
contract. Preserve the complete owned native, Closure, Railroad, Swimlane and
other diagram runtime/test/configuration inputs outside explicitly named Venn
production and API-fixture changes. No new crates.io dependencies; base ESTree
and repository-owned types remain the canonical boundary.

Source values resolve exactly as the incoming producer: the first selected init
directive object replaces the entire front-matter config object, then missing or
invalid individual values fall back to defaults. They do not inherit secondary
front-matter fields. Defaults are width800/height450/padding8/useMaxWidthtrue/
useDebugLayoutfalse. Booleans are transport and metadata, with no geometry branch.

Scope Venn directive lookup to actual closed single-line init directives (the
forms admitted by this line grammar). Recognize a quoted `venn` object key with
a colon and immediate object value, ignoring quoted value/string contents.
The incoming whole-source helper can otherwise mistake a quoted set name and
style object for canvas configuration, or a quoted `venn` value followed by an
unrelated object. Add exact no-directive/quoted-body/non-init/malformed-init and
quoted-value controls. Keep all other dialect extractors byte-exact; this bounded
Venn path is not a general JSON5 parser or a multiline-directive parity claim.

Finite alone is insufficient for this new text-reachable canvas path. The
incoming width=height=1e308 with four equal sets and a labeled union can overflow
the centroid sum even though individual circle coordinates are finite. This is
a numerical witness, not a demonstrated undefined-behavior finding. Introduce an
owned MAX_VENN_CANVAS constant65536. Parser dimensions must be finite in
(0,65536], padding in[0,65536]; invalid/out-of-range values use each field's
default, preserving fallback semantics rather than clamping. This scalar bound
does not certify downstream image-area allocation, strokes or arbitrary styles.

Add try_layout_venn for caller-owned semantic IR. Validate width, height and
padding before arithmetic, including empty and titled diagrams. Return a typed
field error for unsupported values. The incoming single-argument layout_venn
keeps its infallible signature as a compatibility wrapper for supported IR and
panics on unsupported IR; untrusted typed callers use the checked API. Validate
complete derived canvas extents, circle coordinates/radii and label positions
for finiteness and nonnegative extents/radii. Preserve bounded valid geometry
byte-for-byte outside the validation wrapper; do not silently clamp source config.
Effective placement padding remains min(authored,width/4,height/4), while
semantic/layout config and metadata preserve the authored admitted padding.

First capture actual incoming baseline compilation/tests. Add tests before
production repair: source/default/directive/front-matter admission and whole-
object precedence; exact three numeric fields and both booleans; typed empty/
titled invalid scalar controls; concrete huge-canvas witness; independently
tabulated circle/union/text geometry and full configuration; complete glyph,
primitive/style/order and five-key metadata controls for default and custom
canvases. Verify both booleans' exact transport without inventing rendering
semantics. Preserve existing tests and make every owned constructor/call migration
explicit. Invalid region-size derived coordinates need precise rejection tests.

Tiny titled canvases, full label/stroke enclosure, area-proportional overlap,
theme/CSS safety, arbitrary custom-font fitting, forged style values, renderer
pixel-area budgets and general resource bounds remain explicit debts. Passing
finite/metadata or token-presence controls alone does not establish visual parity.

After focused green freeze the source, run the complete metadata-derived
affected fourteen-package parser/layout/paint/downstream suites and strict
all-target lint. Attribute retained native166 package inputs to actual b768
1799/0/1ignore/78 (LANG492) receipts only by complete canonical identity, with
explicit named documentation/local-version exceptions if any; never infer new
native execution. Corrected Closure36/3705 and registry376 identities remain
required. Final README/changelog/backlog, actual documentation/lessons/whitespace,
fresh unforced current-main build plan and independent exact-head source,
terminal and carrier security reviews gate same-PR publication. Current d400
hosted runs remain untouched until a newer reviewed publication exists.

Miri DEFERRED/PENDING, never passed. Preserve its frozen checkout, inputs,
receipts, process and target without polling, termination or duplicate runs.
Deliver remaining implementation first, then genuine final-source Miri. Full
Closure parity, output-byte/source joins, stable IDs, pass/sweep order and all
fold/rename/inline/motion/merge/delete lineage/CV04b remain OPEN; heartbeat ACTIVE.
