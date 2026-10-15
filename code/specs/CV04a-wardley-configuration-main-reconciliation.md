# CV04a: bounded Wardley configuration and checked geometry

Specification before tests/import/implementation. Inspected main6c4e482c
(#17599), parent47ee139e, modifies12 paths/24 actual blobs. Independent static
review225cc760/manifestbce452cf WITHHOLDS raw integration: finite but huge radius/
font settings overflow derived paint values, axisFontSize1 produces stage font
-1, quoted/nested origin/member extraction admits invalid prefixes, and public
typed/body-size ingress bypasses admission. No executed UB is asserted.

Use clean idle R2 on a new branch from native14 sourcee3e1106c. Preserve its
published e16c branch/receipts and isolated R1 source/HEAD/index/Cargo target
while native14 full38995 runs. Source2cc7257b accepts e3 focused50/1/1,60 new
normal-exit cells and strict6, but full native terminal remains PENDING. No
Wardley edit can alter R1's frozen bytes. Use the separate idle Ishikawa Cargo
target for future style execution. Sole implementation PR17288 stays unmerged/
autoOFF; no current-main/hosted/merge/Miri acceptance follows from this spec.

## Preserve the incoming ordinary behavior

WardleyDiagram replaces width/height with required WardleyConfig; resolved
layout clones the complete config. Defaults:900x600,padding48,nodeRadius6,
nodeLabelOffset8,axisFontSize12,labelFontSize10,showGridfalse,useMaxWidthtrue.
Default stages are Genesis/Custom Built/Product/Commodity. Preserve new symmetric
margins,padding clamped to one third of resolved width/height, and resolved
minimum420x300. Preserve metadata for seven non-canvas fields and actual scene
width/height, grid branch/primitive order, label offsets and anchor radius1.35.
Title font is axisFontSize*1.05; stage font is max(axisFontSize-2,1).
Wardley source sizes override caller font sizes as upstream intends; retain
caller family/weight/italic/stretch and exact ordered cached font queries.
useMaxWidth is transported metadata, not a responsive-layout guarantee.

## Own numeric and typed boundaries

Declare MAX_WARDLEY_SCALAR65536 in owned IR. Canvas width/height and node radius
must be finite positive and<=the bound. Padding/node label offset must be finite
nonnegative and<=the bound. Both configured font sizes must be finite in
[1,65536]; lower positive values use the source per-field default or a named
typed error. This explicit font lower bound prevents degenerate text scaling;
it is scoped to new Wardley config and changes no other dialect's admission.
The stage minimum1 retains ordinary12→10 intent and supports axis1/2 safely.
Negative/NaN/infinite/huge values must not reach drawing through a finite-input
claim. Limits apply before empty-map paths too. This is a scalar subset, not an
area/node-count/string/font-fit/resource/containment guarantee.

Add WardleyLayoutError and try_layout_wardley returning named errors for each
config field, invalid normalized node/evolve values or unknown link/evolve
references, and derived invalid geometry. Keep layout_wardley as the documented
compatibility expect wrapper. Typed visibility/evolution/evolve target must be
finite in0..1; validate references before filter_map can silently drop them.
After existing supported arithmetic, inspect complete canvas/node/link/evolution
positions for finite values. Validate configured/derived radius and all font
sizes before returning a supported layout. Caller-forged already-resolved Paint
layouts remain explicitly outside this checked semantic-IR entry unless a later
spec adds a separate check. Do not claim that forged resolved values are safe.

## Own source origin/member/value boundaries

Use a Wardley-local settings adapter over unchanged owned closed-init/direct-
member/header-ancestry/active-quote helpers. Only direct config.wardley-beta
frontmatter mappings or a direct wardley-beta member of a closed init object
can supply fields. Init replaces the corresponding map wholesale, including an
empty object. A present invalid field uses its Wardley default rather than an
earlier frontmatter value or nested decoy. Accept complete numeric/boolean
values, not prefixes ending at the wrong quote: valid JSON width760'junk inside
double quotes is invalid as a number. Keep opposite/escaped quote rules and
malformed-tail refusal consistent with owned semantic adapters. Nested objects/
arrays/unknown ancestors/bare sequences/block scalars/header-literal directives
must not supply settings. Do not rewrite shared settings helpers or other
dialects' consumers. Complete YAML/JSON5 semantics remain outside this subset.

The later body size clause overrides admitted config only when both dimensions
are finite positive<=65536. Invalid body sizes return a source ParseError at the
size line; never silently fall back or bypass the same numeric boundary. Existing
normalized-coordinate/link/evolve source errors stay intact. Preserve defaults,
header/closed-init/body precedence and staged source error ownership explicitly.

## Closed import and exact causal controls

Materialize/verify all24 full parent/after blobs before index mutation. Recompute
the complete expected mode/type/blob tree from the actual owned left and incoming
12 paths. Independently resolve every whole-file overlap; preserve all owned
non-Wardley production/helpers/oracles byte-exact. Migrate only Wardley ABI
constructors and explicitly specified old default/geometry expectations. Keep
37 complete Ishi source/IR/layout/scene/query cells,232 complete Venn scene/query cells in13 fixture files and67
causal parserIR controls exact. No generic coordinate replacement/tolerance
change/deleted assertion/default normalization or borrowed acceptance.

Tests precede guards. Capture real failures for malformed numeric quotes, nested
origin/member decoys, huge finite radius/title overflow, stage axis1 and typed/
body-size invalid inputs. Cover each field/default/zero/tiny/one/two/cap/overcap/
nonfinite boundary; direct/nested keys, quoted-key/value tails, booleans, empty
map replacement and source precedence. Typed empty/nonempty cases assert named
errors and successful complete layout values, not only token presence. Complete
source-to-IR/layout/PaintScene/glyph/metadata/ordered-font-query tables must be
specified independently before observing renderer output. Include defaults,
custom grid on/off, anchors/components/links/evolves, one/two font families and
minimum font behavior. All lowered scalar parameters must be finite and fonts
positive within the specified checked route. No generic visual/fontfit claim.

## Validation and delivery

Recompute affected14 reverse/50 forward normal/dev/build/target graph and all
working raw Rust/TOML bytes,registry376,only4 local version lock changes, grammar
corpus/resource maps and historical Closure316e36/3705+3 static attribution.
Run focused actual controls, complete unfiltered affected14 suites and strict
all-target warnings/unknown-lints denial. Native14 actual full acceptance must
genuinely finish at fixedR1 before carrying its2853 inputs/resources into this
carrier; no Wardley mutation invalidates that run. Independent source/terminal
and final-carrier/helper reviews precede normal same-PR publication. Reconcile
README/changelog/spec/backlog/linked issues; real doc/lessons/diff checks and fresh
unforced current-main plan are required, selection not package execution.

Squashauto only current conflictfree/reviewclean exacthead terminal acceptable
requiredCI; actual fetched merge/payload reachability before cleanup/CV04b.
Actual macOS phase-refusal cause remains OPEN. Base ESTree canonical serialized
boundary/own types/no new crates.io dependencies remain owner decisions. Full
Closure parity/outputbyte/source/fold/rename/inline/motion/merge/delete lineage/
stableIDs/spans/pass-sweep remain OPEN. Miri DEFERRED/PENDING, never passed;
frozen checkout/run/inputs/target/receipts untouched/unpolled, no duplicate or
termination. Delivery first, genuine final-source Miri before full completion
or disabling the ACTIVE heartbeat.

The two positive over-cap body-size forms must produce line2/column1 with the
exact message `size must be finite and at most 65536`; existing nonpositive-size
errors retain their earlier message. Parser testfirst declares94 complete IR
cells (70 scalar/16 origin/6 boolean/2 body) plus2 exact size-error controls.

## Bounded Wardley configuration and checked geometry

Wardley source configuration uses direct owned mapping ancestry or a closed init
object; an init object replaces the mapping wholesale, including empty objects.
Nested/sequence/block-scalar/quoted-tail decoys cannot supply a direct field.
Width/height/radius are finite positive values at most65536; padding/label offset
are finite nonnegative values through65536; configured fonts are1..65536.
Invalid source settings use field defaults. Body size uses the same finite cap,
with a named source error for positive over-cap values.

The checked typed route `try_layout_wardley` rejects invalid configuration,
non-normalized node/evolution coordinates and unknown references before mapping,
and rejects nonfinite derived geometry. Compatibility `layout_wardley` refuses
unsupported typed values. Ordinary default/minimum/cap layouts remain literal
whole-layout controls. Stage fonts use max(axisFontSize-2,1); title fonts use
axisFontSize*1.05. Grid paths, node/anchor radii, source precedence, all glyphs,
seven metadata fields and ordered cached font queries have exact whole-scene
oracles. General font fitting, containment, area/node-count resource guarantees
and caller-forged resolved Paint layouts remain outside this bounded contract.

Spec3b6e precedes closed incoming6c merge8f43 and test-first source/typed/Paint
controls. Semantic reds parser2P3F101/geometry1P3F101/paint0P1F101 are retained.
Actual3600 full strict101/wrapper1 stopped before full tests on negative-fixture
assignment formatting;1b216e6b changes only Rust whitespace plus documentation,
preserving every assertion and independently specified numeric table.

Actual tested1b216e6b: focused5parser/8geometry/2paint PASS;94 complete IR cells,
2 exact size errors,271 geometry events and8 complete source/IR/layout/PaintScene/
glyph/metadata/ordered-query cells. Full affected14 packages981P0F0I/47targets
and strict all-target warnings/unknown-lints denial both exit0. Tests02de82c1/
strict54460559 and independent source/terminal reviews bind that exact source.
All1464 raw Rust/TOML inputs, reverse14/forward50/style827 objects, native2853,
registry376, only4 local style-version changes and resources21+3 are verified.
The13 retained Venn fixture files contain232 complete scene/query cells;
37 complete Ishikawa cells and67 causal parser IR controls remain exact.

Native execution is independently attributed to actuale3e1106c1836/78,
compiler895/fullLANG506/strict6 and146sources/876 six-backend cells. Historical
Closure36/3705+3 conservative static inputs retain316e attribution; no fresh
Closure execution or dynamic/toolchain/OS resource closure is inferred.
Integrated main6c4e is an ancestor. Later inspected main987ab434 Wardley theme
and e0724e0 human-language snapshot changes are NOT integrated/validated here.
This is local bounded acceptance and unmerged progress, with current-main/
hosted exact-head CI/conflict/review/actual merge gates and the actual macOS
phase-refusal root cause OPEN. No TS/book runtime or general visual-parity claim.

Base ESTree canonical/own types/no new crates.io dependencies remain owner
decisions. Full Closure parity/output-byte-source/full transformation lineage/
stable IDs/spans/pass-sweep/CV04b remain OPEN. Miri **DEFERRED/PENDING**, never
passed; frozen run/checkout/inputs/target/receipts untouched/unpolled, no duplicate
or termination. Delivery first, genuine final-source Miri before full completion
or disabling the ACTIVE heartbeat.
