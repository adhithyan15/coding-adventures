# CV04a: two-unit outer-entier reconciliation

Main2fb1841f (#17535) conflicts with published style carrierd02a8dc6 in the
ALGOL compiler. Style source965 remains accepted (871 tests/34 targets and
strict0); native source63 remains accepted only for its own unchanged inputs.
Reuse the clean terminal native checkout on a new branch fromd02, separate
from the accepted style checkout. Its native Cargo target is separate from
frozen Miri. Miri is DEFERRED/PENDING; do not query or mutate its checkout,
inputs, process, target or receipts. Keep one implementation PR, auto-merge OFF.

Commit this spec before importing main or changing tests/production. Capture
the full base/owned/incoming inventory. Preserve every unrelated object and
pure incoming path; reconstruct every overlap. Keep producer version/docs,
test changes, all owned identity/alias/resource controls and independent merge
evidence. Incoming tests for nested two-unit offsets must execute, not be
discarded to make the merge appear clean.

Incoming main widens the shared builtin_sign_operand proof. That function is
also consumed by exp/log chains whose finite-range preconditions must stay
narrow: triple exponentiation at sign(x)+2 can overflow. Preserve that shared
helper byte-exact to accepted4c/63; do not restore a shared unchecked/counter
scheme or widen other helpers. Admit the new feature only in the dedicated
outer-entier proof. Keep existing dispatch, exact read-once NameReal declaration
and resolved-identity/body/signature checks, effect invalidation, conversions,
call fallback and formatter predicates byte-exact unless a demonstrated defect
requires a separately specified change. Never revive cached proof forwarding.

Preserve the existing one-unit flat helper as a named legacy fast path, called
once at the new proof entry. New bounded composition uses one iterative
worklist and one cumulative absolute-unit budget <=2. The legacy attempt also
runs at each zero-budget conditional-arm entry, preserving old constexpr
one-unit arm positives. It never runs inside the new worklist. Signed/parenthesized and
token-free parser wrappers do not reset it. Accept additive +/- structure and
unshadowed unary abs/entier only; reject multiplication/division/powers or
unknown calls in the new additive composition. Its only new dynamic base is
a resolved unshadowed direct sign(actual), surrounded by the explicit supported
wrappers; arbitrary old bounded functions plus two are not newly admitted.
Recognize wrappers and additive structure before base matching, avoiding a
whole-subtree rescan at every wrapper. Require exactly one such dynamic root.
Multiple dynamic roots, noncanonical static subtrees and variable offsets
remain refused; none may be rescued by the old classifier inside a new form.

New offsets use canonical integer/real literal values0,1,2 with unary signs;
sum their absolute units with checked arithmetic. Do not use generic f64 static
folding as exact integer semantics: ((9007199254740992+1)-9007199254740992)
rounds differently under floating and integer evaluation. Conservatively refuse
noncanonical new constant compositions rather than silently extending that
generic-fold proof. The inherited legacy helper/folding behavior remains a
separate debt, not a new exact-integer-fold guarantee.

Retain the one conditional front with independently bounded then/else arms,
each <=2 and both strict local roots. Do not sum disjoint arm budgets. Common
offsets outside a conditional and arbitrary nested conditional composition stay
refused, preserving the bounded front-only contract. Actual lowering must retain
condition/branch name-write invalidation and value-write preservation; no saved
proof may survive a mutating actual. Exactly one chosen branch executes.

Tests precede repairs. Record imported compiler/runtime reds with precise
diagnostics; compile failures are distinct from executed semantic failures.
Extend exact formatter-call and Type-refusal tests for +/-2, flat/nested units,
abs/entier wrappers and both conditional outcomes. Move intentionally newly
supported two-unit cases from the old refusal matrix into positive controls;
replace the refusal boundary with >=3 and wrapper-spanning budget resets.
Retain shifted narrow exp/log refusals unchanged, all malformed identity trees,
variable/two-root/shadowed builtin controls, read-once call counts and8192
classifier discovery limits. Include the large-integer round-trip trap in a
new two-unit context, so a generic f64 fold cannot hide a third unit.

Run exact stdout and normal-exit controls across every available standard
backend, including saved-local results and ordered condition/selected-branch
markers with unselected branches absent. Preserve all prior16sources/96cells;
add new two-unit controls separately. Native availability is actual execution,
not merely a declared backend; unavailable CLR is not counted as passed.

After focused/library/runtime validation, freeze final source and run the full
affected six-package native/compiler/LANG/downstream suites and strict lint.
Recompute the complete accepted-style diagram forward dependency inventory;
carry source965 execution only if all runtime/test/config inputs remain exact.
No new crates.io dependencies, registry identities/checksums unchanged. Update
README/changelog/backlog/issues, documentation checks/current build plan and
independent exact-head source/carrier security review before same-PR push.
Only current-head terminal acceptable required CI plus review-clean/conflict-
free state permits squash auto-merge. Verify actual fetched-main reachability
before cleanup. Full Closure parity, output-byte/source joins, stable identities
and every transformation lineage/CV04b remain OPEN. Finish genuine final-source
Miri after implementation delivery; heartbeat stays ACTIVE until then.
