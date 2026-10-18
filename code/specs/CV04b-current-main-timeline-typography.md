# CV04b — Timeline font requests and paint typography

This bounded current-main reconciliation follows font-size ingress at
`cacc0c77e473afec6079eb89c88d15065e1b3b7b`. Miri is DEFERRED/PENDING; the frozen
run, checkout, inputs, receipts and target stay untouched and unpolled. Full
Closure parity, transformation lineage, output-byte/source joins, current-main
integration, hosted checks and actual merge remain OPEN.

## Observed operations and scope

The owned temporal painter carries config.font_size but still paints with
caller label/title sizes. The incoming painter overrides ordinary temporal
fonts even without a Timeline config, doubles title size without admission,
and changes period font roles. Adopt configured Timeline sizes on the guarded
Timeline font clones only. Preserve the existing title role for TemporalTitle
and TimelinePeriod, and the label role for TimelineSection, TimelineEvent and
embedded period events. Period-role reconciliation, size-driven layout/reflow,
section grouping and incoming width changes remain separate obligations.

With Some(config), label size is config.font_size and title size is exactly
config.font_size * 2.0 in f64. Preserve caller weight, italic, stretch and line
height, existing effective family selection, fixed incoming text frames,
shape geometry, colors, accessibility and instruction ordering. With None,
keep complete caller behavior. A mixed diagram's ordinary axis, Journey and
generic item paths keep their existing fonts and numeric formulas. Add one
timeline.fontSize metadata value containing the transported f64's to_string;
do not advertise that the emitted f32 glyph size equals every authored f64.

The actual temporal bridge uses device_pixel_ratio=1.0 for text independently
of the caller's scene option. layout-to-paint casts font.size to f32 before
TextShaper::shape, while its metrics/baseline calculations still use f64 size.
FontCacheKey excludes size and line_height: different sizes with the same
family/weight/style/stretch share resolution. Keep that cache behavior.

## Checked request admission

Introduce TemporalPaintError with TimelineFontSize and TimelineTitleFontSize
variants, plus try_diagram_to_paint_temporal with the current arguments and
Result<PaintScene, TemporalPaintError>. The existing public function delegates
and retains its PaintScene signature, explicitly panicking with
"unsupported temporal typography" if this new checked boundary refuses input.

For Some(config), before instructions, text nodes, resolution, metrics or
shaping, require raw label size to be finite and positive and its f32 cast to
be finite and positive. Otherwise return TimelineFontSize. Then compute the
doubled f64 title size and apply the same f64/f32 admission, returning
TimelineTitleFontSize on failure. Validate both planned roles even for empty
diagrams. No arbitrary magnitude cap, clamp, silent fallback or source rewrite
is permitted. Font-size ingress and checked layout still admit every positive
finite f64 exactly; request admission is a distinct later operation.

This certifies two representable positive font requests only. It does not
certify third-party metric/shaper output, accumulated glyph coordinates,
caller line-height arithmetic, clipping, font resources or rasterization.
Do not claim a general finite-scene or rendering-resource guarantee. None
does not gain a new font/geometry validation policy in this slice.

## Exact evidence and retained controls

Commit this specification before tests or implementation. New tests first
compile against an ABI-only checked wrapper with old paint behavior, producing
actual semantic failures before the numeric checks and size binding. A missing
symbol or compiler failure does not count as a regression red.

Exercise exact error variants and zero resolver/metrics/shaper calls for raw
nonfinite/nonpositive input, f64-positive values rounding to f32 zero, f32
overflow and doubled-title overflow. Include boundary neighbors whose label
request is representable while the title is not; positive f32 subnormals and
fractional values must remain supported where both requests are representable.
Compare checked and compatibility entry points on accepted cases, including
empty Some, ordinary None, both directions, title/section/period/event/embedded
events, distinct and merged query identities, and nondefault caller attributes.

Declare complete Scenes and ordered resolver queries before production calls.
Use the existing fake backend's independently stated half-em advances and
800/1000 ascent, including its actual f32 shaping width and retained f64 metric
operations. Declare explicit wrapping cases and hard-line advances; never
assume changing font size preserves old line breaks. New fixed text frames
are not inferred from returned production output.

Preserve every original authored source and complete IR/layout expectation.
Preserve the entire original legacy Scene data. A closed, reviewed expected-
Scene migration may replace only the declared glyph runs and add fontSize
metadata, using declared layout frames and explicit independent text formulas.
It must retain all original nonglyph instructions, other metadata, dimensions,
background, id and ordered query controls. Never normalize actual output.
Retain whole before/after bodies and bidirectional recipes for every changed
existing test file. None and unrelated temporal controls remain exact.

Recompute the manifest graph and actual affected target/doctest census. Run
affected native/compiler/downstream tests and strict all-target lint, obtain
independent exact-source and actual-terminal security review, and reconcile
README/changelog/backlog/linked issue evidence before publication. Keep one
active shared PR and preserve the current conflict-free/review-clean/exact-head
terminal hosted-check gates before auto-merge; verify real merge reachability.


## Bounded execution and numeric clarification

After label admission, f64 doubling cannot overflow and the positive title
cannot underflow: the reachable title refusal is its doubled f32 cast. Positive
f32 subnormal requests can produce zero half-em fake advances; admission does
not promise a nonzero advance or finite accumulated Scene coordinates. Raw
values slightly beyond f32MAX can still round to finite f32MAX. Exact title
transition label neighbors 0x47dfffffefffffff and 0x47dffffff0000000 are retained
as declared acceptance/refusal controls. These clarify the original request
contract without adding a magnitude cap or changing existing layout admission.

Executed `dba1ff58392bc8e1a721b52d0de4e230632f9efe`:1186 native+2 doctests/0failed/0ignored/0filtered,
59 unfiltered harnesses,14 doctest libraries,14 strict checks. New40 cells are
26 exact errors/zero-work probes,9 empty complete Scenes,4 whole wrapped Scenes
with fractional f32/f64 and caller-style/line-height controls, and1 whole mixed
Some axis/Journey/Timeline Scene, plus the compatibility panic. Every original
source, complete IR/layout, legacy Scene body and ordered query control remains.
Expected glyph migration checks the old complete GlyphRuns and both declared
fit conditions before substituting new expected glyphs; actual output is never
normalized. Compiled fe15 red173P11F preceded implementation; eab compile E0277
is separate preserved evidence. The original green wrapper also remains EXIT1
for its phantom ninth retained priority marker; all88 actual commands exited0.
Separate read-only adjudication validates the real104 markers, every raw stream,
artifact and frozen input; it does not reexecute Rust. Execution SHA256 `9e41e69d11c83fd623cb1c160d8cf0c5082967d7921473192100f0df2e61feaa`;
source review `384dbe33397020465998b0c8db9c7b57501c0aed464ace2d4703dd713ec21cf4`; terminal review `a37c666c927a1cab1862bb04344080e5805d7bd950d616817a0610430558243a`.
Painting now uses size; size-driven layout/reflow, period-role/main integration,
hosted/merge/full-goal gates and genuine final-source Miri remain OPEN.
Miri DEFERRED/PENDING; frozen run/inputs/receipts/target untouched and unpolled.
