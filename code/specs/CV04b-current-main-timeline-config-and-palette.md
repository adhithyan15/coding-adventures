# CV04b: owned timeline configuration, checked frames and palette transport

This spec precedes integration/tests/implementation of timeline94df and palette
ac29edea24d77da03217a3502bfb05d2f6d04139. Their source audits95041495/bf258a86
remain WITHHELD. Their full immutable blobs are preserved. Ordinary authored
layout arithmetic and intentional new palette defaults are retained; no existing
native/compiler/Closure/diagram receipt accepts this incoming source.

## Owned configuration selection

Change only timeline entry points and a small local adapter. Reuse the existing
owned Venn envelope/object-member/parent-preserving frontmatter/active-quote
helpers without changing their bodies or other dialect callers. No global
mermaid_directive_object/quadrant reader rewrite or new parser dependency.

The selected init root is the FIRST genuinely closed single-line
%%{init: {...}}%% root recognized by venn_init_root's current semantic envelope.
It skips leading blank lines/YAML header content and requires the complete
closed payload/trailing directive delimiter. Ordinary %% comments, nested objects,
quoted semantic text and literal directive-looking YAML block content cannot
supply settings. Later init roots do not overlay the selected first root.

Before that root selection, exclude physical lines OWNED by successfully parsed
ACC_DESCR_BLOCK tokens. Existing global preprocessing blanks directives before
multiline accessibility tokenization, so raw lines inside description data can
otherwise masquerade as init roots. The timeline-local adapter derives inclusive
physical line intervals from each trusted token's1-based start line and checked
newline count. Header/directive preprocessing preserves physical newlines, so
these intervals refer to original raw-source lines. Handle LF/CRLF, inclusive
closing lines and adjacent/overlapping intervals in one ordered source/interval
traversal, O(source bytes + tokens), not a whole-source rescan per block. Blank
all non-CR/LF bytes of excluded lines to ASCII spaces only in a local config-
selection copy; preserve lengths/newlines/UTF-8 validity. Read both object
families from that copy. Original tokens, source preprocessing, accessibility
content/metadata and all shared readers stay unchanged. Failed source/token
parsing remains an error; do not guess intervals from malformed text.

Current timeline tokens have no other multiline semantic token: title, single-
line accessibility, periods and events exclude CR/LF. Re-audit that fact if the
grammar changes; a physical line cannot be simultaneously an independently
selectable whole-line init root and partly owned by a multiline description.
Add complete-source/IR controls for BOTH timeline/themeVariables init impostors
inside descriptions, Unicode and CRLF boundaries, plus a later genuine init
root proving excluded data does not consume first-root selection. Preserve the
actual current accessibility result after existing preprocessing; no new claim
that directive-looking raw description text was previously retained verbatim.
The v1 contract/source-composition finding e1f29b91 remains WITHHELD and preserved;
this adapter clarification precedes all tests/implementation.

For each object family, select ONLY a direct root member: timeline or
 themeVariables. A missing member permits its YAML object fallback. A PRESENT
closed object, including {}, replaces that entire YAML object. PRESENT nonobject
or malformed member values select an invalid object and use family defaults,
not the YAML object's values. No per-field YAML fallback beneath a selected init
member. The supported YAML paths are exactly config.timeline and root
 themeVariables, retaining the incoming authored subset. Preserve actual parent
mapping/sequence/scalar indentation; nested/unknown maps, sequences and block
scalars cannot lift descendants into these direct scopes. No new YAML generality
or JSON parser conformance is claimed. Duplicate direct fields use the existing
first-direct-field policy; no raw whole-source search is allowed.

Decode scalar values with the existing matching-active-quote/backslash policy,
including complete closing-quote/tail validation. Other quote characters are
data; malformed PRESENT values cannot become a valid prefix or reveal YAML
fallback. Source padding is a complete finite nonnegative f64, otherwise default50.
Source booleans are complete case-insensitive true/false, otherwise false.
Source colors use the unchanged lexical sanitizer per slot, otherwise that slot's
new default. Unknown alphabetic lexical color names may still have renderer
semantic fallbacks: this is not a full CSS/renderer validity or injection claim.

Keep the three complete six-slot default vectors from ac29 verbatim: cScale0..5,
cScaleLabel0..5 and cScaleInv0..5. Configuration readers replace their respective
object families independently. YAML root themeVariables and init themeVariables
must each have full18-slot tests, not only slot0 presence checks. Init timeline
selection does not suppress YAML theme fallback when themeVariables is absent,
and the converse holds. Full IR equality pins selection, colors and booleans.

## Checked temporal API and error priority

Add own enum TemporalLayoutError with TimelineCanvas, TimelinePadding and
TimelineGeometry variants, plus public try_layout_temporal_diagram returning
Result<LayoutedTemporalDiagram, TemporalLayoutError>. Existing
layout_temporal_diagram delegates to it and has an explicit compatibility
expect("unsupported temporal geometry") policy. Source-to-layout tests use the
checked API and propagate Err before painting. There is no separate production
source renderer caller to silently swallow the error; audit every call site.
Gantt/Git/Journey dispatch and separate event-model paths retain existing bodies
and behavior, wrapped in Ok. Do not claim new numeric safety for those dialects
or for forged LayoutedTemporalDiagram/Paint payloads.

Timeline admission priority is raw canvas finiteness, then typed padding, then
coupled derived geometry in original construction order. Raw finite negative/zero
canvas still normalizes to min560 through the incoming max operation; NaN/Inf
returns TimelineCanvas. Typed padding must be finite and >=0, else TimelinePadding;
source-invalid scalar fallback50 remains the parser policy above. A finite
source padding1e308 may therefore be present in IR but must produce
TimelineGeometry when actual geometry cannot be represented. Do not hide an
invalid derived case by silently clamping padding or changing direction/title.

Check every scalar calculation used by a timeline item and the final canvas for
finiteness, including multiplication/addition/subtraction and color ordinal
increment. Verify every positive rectangle/frame dimension AND increasing actual
represented x+width/y+height endpoints. In particular validate the painter's
actual text frames, preserving its existing arithmetic:

- TemporalTitle: x+8,y+6,width-16,height-12.
- TimelineSection: x+10,y+6,width-20,height-12.
- Period title: x+12,y+8,width-24,height22.
- Nonempty event text: x+12,y+32,width-24,max(height-38,12).

Validate both outer positive item rectangles and all used inner text frames;
zero-length spine lines are allowed, but both endpoints must be finite. The
final width is positive/finite; final height is nonnegative/finite, with zero
height allowed only for an empty zero-padding timeline without items. Validate
the final represented item/text extents, not merely a centered intermediate.
Fail locally before publishing a result or creating a Paint scene; no partial
layout is returned. Locally owned construction and existing linear input/label
allocation are not a universal fallible-allocation or physical-resource promise.

No blanket maximum canvas/padding/font/node-count is introduced. Derive validity
from the actual chosen direction/items: padding1000 at640 with TD periods refuses
negative frames, while LR without a title may legitimately expand its canvas and
remain valid. Keep that large-padding LR positive control; a title can invalidate
its own frame independently. Preserve incoming TD/LR positions, spacings, title
and card dimensions for supported cases. No font-fit or containment guarantee.

## Spine, work and palette semantics

Compute without_sections once per layout and reuse it in both directions; do not
repeat an all-sections scan inside every LR section. TD spine presence follows
whether ANY period was emitted, independently of disableMulticolor or palette
ordinal. Preserve its original placement/order and endpoints. LR keeps its
per-nonempty-row spine, including the valid one-period zero-length line.

Retain palette ordinal behavior: it advances for every period unless the entire
diagram is unnamed and disableMulticolor=true. Section headers record the current
ordinal; headers alone do not advance it. Each of the three typed palette vectors
cycles independently by its OWN length, preserving empty-vector fallbacks and
usizeMAX indices without modulo0. Arbitrary typed vector lengths are not silently
truncated or normalized to six. All configured vector entries remain in metadata.
Do not clone/drop authored palette entries to conceal unsupported cases.

Use cfg(test)-only work observation on the predicate/section traversal to prove
linear work for many public typed unnamed sections (including empty rows).
There is no new production comparison counter/cap, and no false claim that parsed
source currently constructs the same many-unnamed-section shape. No new output
complexity claim beyond the retained linear input/output traversal.

## Exact retained oracles and delivery

Before merging, preserve complete current-owned/raw-parent/raw-new bodies and
closed overlap recipes. Restore the old named-section configured e2e source
alongside the new one; do not replace or discard it. Every public TimelineConfig
and TimelineSection item constructor/pattern migrates explicit required fields
with a full-body inverse, without default overwrites erasing authored values.
Default palette changes intentionally revise only independently declared style,
glyph-color and metadata oracles; preserve sources, geometry, fonts and queries.

Tests first: genuine compiled source ownership/derived-frame/spine/work reds,
then repair. Complete new source->IR->checked TD/LR layout->whole ordered
PaintScene/all glyphRGBA/ordered cached font queries/full metadata/accessibility
oracles cover default/custom18slots, >6 cycling ordinals, named/unnamed/empty/
one/multiple sections, both booleans and title states. Typed palettes0/1/2/6/8,
independently mismatched family lengths, empty fallbacks and large indices retain
whole-value controls. Keep every prior complete Venn/Wardley/Ishikawa scene and
query assertion, font/frame/resource guard and non-timeline temporal test.

Add NaN/Inf/negative typed padding/canvas cases, huge finite source padding and
large-padding direction/title companions, represented-extent precision refusal,
empty zero-height positive, explicit wrapper panic and checked error priority.
Independent layout/scene tables must be declared before first rendering; expected
values never read actual returned layout/production painter helpers. Unique
post-assertion markers and executable joins bind the complete cell census.

Use the isolated worktree/separate target, reconcile only required local lock
versions while preserving external records, derive fresh complete affected/input
inventories and run affected full suites/strict. Exact-head independent source
and terminal security plus current conflict-free review-clean exact-head terminal
acceptable required hosted checks still gate delivery/auto-merge. Keep one existing
PR, verify actual main merge reachability before cleanup. Miri DEFERRED/PENDING,
never passed; frozen checkout/run/inputs/receipts/target untouched and unpolled.
Full Closure output-byte/source and transformation-lineage completion stays OPEN.
