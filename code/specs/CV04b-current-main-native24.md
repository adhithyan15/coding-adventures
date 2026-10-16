# CV04b: reconcile incoming24 with the dedicated exact-unit proof

This contract precedes integration/tests/implementation for incoming
6e3b5ac2732941d5753cd56e256f2efbdfe3d5fa. Source auditb8b5dc89 is WITHHELD;
raw shared exp/ln widening and generic f64 arithmetic are not accepted. Existing
source8cff retains the bounded cap23 production and all earlier receipts. No
runtime/native/Closure/Miri acceptance transfers to the incoming commit.

## Production boundary

Change only four dedicated proof locations in algol-iir-compiler: the explanatory
23/[-24,24] comment becomes24/[-25,25], INTEGER_LIT upper bound23 becomes24,
REAL_LIT's exact canonical table gains24, and checked cumulative absolute units
are limited to24. Reconstruct the inverse by these four changes over the entire
production prefix; every other production byte, including the narrow shared
builtin_sign_operand, exp/ln consumers, single-unit fallback, conditional front,
root/effect/read-once/alias/shadow/formatter and generic f64 rules, stays exact
owned8cff. Incoming three shared-helper edits are excluded explicitly in the
closed merge recipe. Existing cfg(test) observation hooks remain inert in normal
production. The8192 visit assertion is an observer, not a new production limit.

The dedicated worklist admits canonical literal units0..24 and one direct sign
root per existing proof arm. Every additive/subtractive operand contributes its
absolute literal units; cancellation cannot refund units. Parentheses, unary,
abs and entier wrappers cannot reset the cumulative sum. No recursive fallback
beneath a new additive/wrapper item is introduced. Current conditional-front
and strict post-condition branch-root rules stay byte-exact: conditional beneath
an added literal is still a structural refusal. Multiple roots, opaque/dynamic
constants, shadowed calls, unsafe effect/alias/root changes and unsupported
operators retain their refusals. A generic f64 fold cannot certify exact integer
units: preserve both unrepresentable-literal and intermediate2^53 rounding
counterexamples. ((9007199254740992+1)-9007199254740992+24) has exact25, even if
binary64 folding yields24. No new crates.io dependency or unsafe code is needed.

## Complete source retention and test-first proof

Before touching sources, derive a complete occurrence inventory from current
owned Rust tests, incoming parent94df and incoming6e3b. Bind full literal source,
function, original assertion/oracle and count. Compare decoded whole sources,
not substring counts or conflict markers. The read-only current presence receipt
for seven raw lost contexts (2/2/1/1/1/1/1 fullsubstring hits) is only a locator;
it is not an AST/oracle/admission census and must not create duplicate tests.

The independently decoded current inventory contains365 negative occurrences in
31 functions:137 pure quantitative24 occurrences (29 complete source identities)
and228 unchanged safety occurrences. The137 are126 formerly migrated rows,
TWO historical rows29/30 previously grouped among221 kept cases, four original
main additive contexts, four canonical generator contexts and source(24) in the
integration test. The two historical duplicates have distinct occurrence IDs;
they are quantitative at the new boundary, not permanently structural guards.
The old126 cap22 positive occurrences remain byte-exact. Materialize the four
canonical generator programs only with a full-function inverse; integration
source(24) becomes source(25) with its exact diagnostic oracle retained. Every
new24 companion records its original occurrence ID and declared mathematical
result independently; a source-level result is not a claimed executed backend
observation. The raw seven contexts all map to existing decoded occurrences;
restoring raw positives may not replace any owned function or erase an oracle.

Retain every current negative occurrence and all126 original cap22 formatter
positive companions, plus all later complete contexts. Pure quantitative24
negatives move to individually inventoried25-unit refusals, while their EXACT
original full24-unit sources become real-formatter positive companions. Derive
the numeric expected result independently for each original complete program;
keep sources, producer values, consumer calls/assignments/branches/output and
error assertions intact. Do not replace every24 digit globally: that would erase
structural and effect guards, or change unrelated producer/consumer values.
Whole-file inverse proofs must reconstruct the current files after removing
only documented new blocks and reversing only declared quantitative edits.

The raw incoming missing-source inventory identifies six pure24-unit contexts:
two conditional/local pick2.25 consumers whose mathematical result is25 and four
assigned real-result pick-2.25 consumers whose result is23. Keep their exact
programs, even if already represented by a current occurrence. Count distinct
occurrences and source identities separately; deduplicate new companions only
when the existing complete assertion actually provides the same oracle. The
seventh source is conditional-under-addition23+1 and stays an EXACT structural
negative, alongside incoming24+1. New incoming positives with other producers
or frames do not replace any of these complete sources.

First add focused positive24-unit and negative25/safety tests with literal
sources/output expectations; execute a genuine compiled cap23 red. A compile
failure is not the semantic red. Then perform the four-place dedicated repair.
Run focused cases, the complete unfiltered compiler suite and strict all-target
lint with an independent post-assertion occurrence census/artifact join. Preserve
all prior failed/accepted receipts under their own execution heads.

## Runtime and delivery boundary

Keep all29 existing runtime sources and their original174-tuple/26-Float receipt
attribution. Add independently declared effect-marked24-unit sources, both
conditional outcomes and exact raw incoming choosefalse/right3.5 +24/12+12
source. Keep a25-unit unsupported formatter boundary as genuine compile refusal;
do not invent executed backend tuples for a program that cannot compile.

For real-valued entry programs retain the reviewed distinction between typed
return and host process status: fresh guarded VM/JIT Float bits must equal the
source's exact real result; Wasm and available CLR must satisfy the specified
numeric sentinel contract; bounded NativeAot/LLVM/JVM process0 proves normal
completion/stdout, not their unresolved typed-main ABI. Reject arbitrary nonzero
status, traps and absent typed observations. CLR absence is uncounted. Preserve
existing effect/output ordering and repeated-run isolation/adversarial guard
checks; no source rename or result-erasing expected value is permitted.

Use the existing isolated worktree and separate Cargo target. Derive fresh
complete native reverse/forward and conservative input/resource/lock inventories
after integration; package-only lock version changes must leave registry records
exact. Run affected compiler/native/downstream/Closure checks and exact-head
independent source/terminal security before updating only the existing PR.
Current conflict-free review-clean exact-head terminal acceptable required CI and
actual merge reachability still gate auto-merge/cleanup. Miri DEFERRED/PENDING,
never passed; frozen checkout/run/inputs/receipts/target untouched. Full Closure
parity and output-byte/source/full transformation lineage remain OPEN.

The first compiled cap24 run atbb838 passed all1002 compiler package tests,
including137 complete original-source VM/output/type observations, but strict
Clippy rejected the canonical generator's now-redundant &source borrow. Remove
only that borrow after materializing String generators to &str arrays; the full
assertion/diagnostic oracle and all four complete sources remain identical.
Record this mechanical inverse separately; the first combined receipt stays
unaccepted until a genuine corrected-source test/strict run.
