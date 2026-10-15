# CV04a — isolate the outer-entier unit-offset proof

Independent4a report1ae4761b withholds the incoming unit-offset admission:
parenthesized additions reset its local offset budget and a widened shared
helper leaks into triple-exp/ln range proofs. Actual frontend/LLVM probes
confirm acceptance. Conversion range/trap guards remain; no generated-program
runtime failure or UB is claimed. The diagnostic full native baseline79530 was
stopped after this finding (actual EXIT1, incomplete, never accepted); preserve
its partial logs and six-owned-process termination proof. Frozen Miri was not
in that tree and remains untouched. Run full required suites on repaired source.

Restore builtin_sign_operand's additive-zero-only semantics exactly. Add a
dedicated outer-entier unit-offset operand proof at only the four existing
entier admission sites: selector root, standard selector root, selector runtime
real value and runtime-real assignment. Retain their existing root/both-branch/
post-selector snapshots, builtin identity, classifier/depth/parser budgets,
formatter and conversion checks. Old exp/log/abs/sqrt range helpers must never
consume a broadened recursive result.

The dedicated helper scans the old narrow helper once. Otherwise normalize
transparent parentheses/grammar/unary +/- wrappers once, then recognize one
flat additive sequence: exactly one dynamic operand classified by the OLD
helper, static constants in {0,+1,-1}, at most one nonzero constant, operators
only +/-. Never classify a dynamic operand using the new helper itself. Thus
parenthesized or entier/abs-wrapped inner shifts cannot reset a budget. Preserve
incoming single flat unit cases, reversed operands, parentheses, unary signs,
saved/nested-sign local-name actuals and zero-offset nonlinear controls.
Count normalization/classification visits without rescanning the whole narrow
expression at every wrapper. Preserve existing work bounds without increases.

This closes the two reported OLD-helper proof leaks. It is not a blanket
nonlinear refusal guarantee: inherited generic Boolean runtime-real formatter
composition may still admit outer exp/ln around an approved entier or saved
runtime-real value without retaining proof kind/bounds. That separate generic
composition/range debt remains OPEN. Do not claim general finite runtime or
range acceptance. Broader dedicated wrappers need explicit proof metadata.

Commit exact Type/formatter negative controls before production: flat/nested
two offsets, entier/abs inner shifts, shifted triple-exp/ln, variable/non-unit
terms and overridden builtins. Require CompileError::Type and the existing
cannot-print-real message, not an arbitrary parser error. Preserve accepted
flat plus/minus and original zero-exp controls; original incoming tests remain.
Add owned runtime controls for direct/saved values, signs -1/0/+1 and both
selector outcomes. Assert exact stdout and normal process exit0, explicitly
distinguish unavailable toolchains and require actual execution per source.

Scoped independent draft design PASS c38091d1, with its generic-composition
caveat applied above; this is not implementation acceptance. Run focused red/
candidate controls, all affected compiler/native/downstream suites including
unfiltered LANG, strict lint, docs/carrier/build-plan checks and independent
exact-head security. Reconcile freshly audited main after a closed frame; new
titleTopMargin admission/config adapters require their own controls. Keep one
implementation PR, no new dependencies/unsafe/limit weakening/ignored controls.

General resource, full Closure parity/output-byte source/full transformation
lineage/pass-sweep/stableIDs/spans/CV04b/whole objective remain OPEN. Miri
DEFERRED/PENDING, frozen497 run/checkout/inputs/target/receipts unpolled and
untouched; separate delivery checkout/Cargo target only.
