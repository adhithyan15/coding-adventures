# CV04a: bounded Railroad theme fallback reconciliation

Current main981892b5 (#17545) adds Railroad themeVariables fallbacks after the
four-unit native reconciliation. Commit this spec before import/tests/repair.
Reuse the separate terminal style checkout on a new branch from nativec0f2;
keep R1 source/HEAD/cargo target fixed while full six-package validation runs.
One active implementation PR17288, auto-merge OFF. Miri DEFERRED/PENDING, never
passed; frozen497 checkout/inputs/target/run/receipts remain untouched/unpolled.

Frame all seven incoming paths and reconstruct every overlap. Retain producer
corpus/spec/version/parser and Apple renderer tests, all owned numeric bounds,
whole font/glyph/path/style/spacing controls and native work. Audit existing
config parser and helpers before adding anything. Carry nativec0f2 only with a
complete canonical manifest-path dependency inventory, not new execution.

Theme fields resolve to typed RailroadConfig before explicit Railroad style
overrides. Preserve producer theme-specific primary/secondary/default string
fallbacks, independent marker/line fields, font-family and px parsing, init
directive precedence and both supported theme front-matter locations. This
slice does not promise arbitrary CSS/YAML or fieldwise merging of init/YAML.

The inherited explicit-font parser enforces finite0..65536 and positive font
size, but its missing-value path returns the supplied default directly. A
finite positive theme font above65536 can therefore bypass that admission.
Require the selected effective font size to satisfy the same cap, with the
existing exact fontSize parse error at line1/column1. Explicit valid fontSize
must override an excessive theme font without refusal. Keep the producer's
fallback-to-default behavior for unparseable, nonfinite or nonpositive theme
font values; do not alter explicit-field refusal domains/error precedence.
Admit selected positive finite values through65536 without clamping.

Tests precede repair. Capture raw producer behavior and reproduce selected
theme font cap bypass with exact errors. Cover all four Railroad dialects,
init and both YAML locations; full configurations rather than token presence;
valid/tiny/cap theme numbers with and without px; excessive finite theme
numbers; explicit override precedence (valid/invalid/zero), primary/secondary
color fallbacks, independent marker/line overrides and unchanged defaults.
Portable paint tests compare complete ordered commands/glyphs/shapes/styles/
metadata against independently resolved expected configuration/trace values.
Apple-only raster tests are retained without claiming local execution.
Preserve all earlier explicit numeric bounds and resource/safety matrices.

Freeze repaired source and run affected fourteen-package parser/style/paint/
downstream tests plus strict all-target lint in the separate railroad target.
Recompute all forward input trees by resolved Cargo manifest path,376 registry
identity/version/source/checksum tuples, and nativec0f2 input retention. No new
crates.io dependencies. Final documentation/backlog/issues/checker/lessons,
current unforced plan, correct Closure36/3705 historical1093/181+strict input
attribution and independent exact-head review gate same-PR fast-forward push.
Current conflict-free/review-clean terminal acceptable required/native hosted
checks alone permit auto-merge; actual fetched-main merge reachability gates
reconciliation/cleanup. Base ESTree/owned types remain binding. Broader visual/
resource/forged-layout/floating-hop debts and whole Closure parity/output-byte
source/stable identities/pass-sweep/all transformation lineage/CV04b stay OPEN.
Genuine final-source Miri follows implementation delivery; heartbeat ACTIVE.
