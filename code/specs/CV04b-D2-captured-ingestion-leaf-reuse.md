# CV04b D2 — reuse consumed ingestion identities in the typed compiler

The opening state is the pre-test contract. Its terminal appendix below records
the actual implementation and supersedes earlier pending status.

This specification precedes successor tests and implementation. D1 was actually
published to sole PR17288 at a6047612; its tested source a8d79bab passed strict
lint and1,100 unfiltered tests across180 targets. D1 records the actual consumed
source inventory but still invokes a second CV tokenizer for typed compilation.
That produces disconnected rootless operand identities. Current origin/main
96285de8 and the parser/ASI/bridge/fold audit confirm the existing CLOC27/CLOC31
leaf factories and checked primitive folding are real. The epic's allNone
assessment is stale. D2 connects that existing implementation to D1 ingestion.

Keep the same isolated R4 worktree and its separate cv04b-inventory-cargo-target.
No producer is active. Preserve all D1/source/native receipts and unrelated
primary edits. One remote Closure/shared PR, no new implementation PR. Owner
decisions remain base ESTree canonical serialized boundary, own types and no new
crates.io dependencies. Miri is **DEFERRED/PENDING**, never passed; its frozen
checkout/run/inputs/target/receipts remain untouched and unpolled.

## Private trusted route and ownership

Only the actual traced run_compiler ingestion path uses the new route. Preserve
the public transform_source_with_cv signature and its source-based legacy
behavior, including callers that supply arbitrary parallel token IDs. It cannot
assert those detached IDs belong to a consumed source. A private implementation
may accept the actual captured owned Tokens for SIMPLE/ADVANCED; public wrappers
pass no capture and continue their existing parser path.

After existing lex/define observations borrow the captured Tokens, transfer the
already-owned D1 token-ID strings into the corresponding Token.cv slots, in
order. Require exactly equal vector lengths and originally unstamped tokens
before modifying any token; reject a broken private invariant as a provenance
error. Move existing strings and vectors; do not clone IDs, retokenize source,
build an unbounded per-piece lookup, infer spans from processed values or extend
D1 reference ownership beyond its precharged storage. No extra source/descriptor
clone or new shared API. Every token field except cv is preserved, including
type_name, processed value, flags, newline state and legacy line/column.

Pass this owned stream directly to the existing public asi::parse_with_asi.
It uses the same grammar, DEFAULT_MAX_RULE_DEPTH, restricted-production pass,
retry/insertion cap and error format as parse_javascript_typed_with_cv. The
bridge then receives existing token IDs unchanged and its existing leaf factory
copies them into literals. Neither ASI nor bridge/shared lexer/parser source
needs to change. Do not create parser-token origins or a second source root.
The actual traced typed compiler must fail closed when capture is unavailable
or missing; it must not silently select the source-based reminting fallback.
Ordinary lexical failure remains a typed Parse failure with no published output.
Non-typed levels and tracing off retain existing behavior.

Typed compilation runs before the string-level defines stage. Therefore the
captured stream is for the actual original source parsed by this route; do not
route post-define strings using stale ordinals. Retain the existing per-token
define observations and exact stage/pass/sweep order. No optimizer, emitter,
compilation policy, resource ceiling or dependency changes belong to this slice.

## Synthetic-token boundary

Existing ASI retries clone Tokens and preserve their cv fields. The synthetic
semicolon constructor explicitly assigns cv=None because it consumes no physical
source bytes. Preserve that policy and all ASI limits. No source-inventory row
may be fabricated for inserted punctuation. Test explicit semicolons, EOF and
right-brace insertion and restricted return-newline handling through actual CLI
output plus operand ancestry. This only connects physical leaves. Stable IDs and
lineage for synthetic/composite nodes remain separate unfinished work.

## Exact evidence and legacy test migration

Commit compiled CLI tests first and retain genuine semantic red execution on D1.
Private unit tests must separately prove the ownership/refusal boundary: assert
the Tokens vector pointer and every moved ID string-buffer pointer are retained,
all non-cv Token fields match, and length/pre-stamped invariant failures leave
the caller's borrowed Tokens unchanged. Exercise missing trusted capture and
ordinary lexical-error capture as distinct refusals. They must never enter the
legacy source-tokenizing route. Preserve a public Some(cv) facade call with
arbitrary detached IDs as an explicit legacy-compatibility control; it still
tokenizes its source and cannot claim consumed-root certification. CLI graph
and output equality alone cannot detect ID cloning or cover private invalid
states. Boundary tests may be committed after the compiled CLI red but before
their production helper exists; compile-failure is not semantic red evidence.

Start at each replacement's own constant-fold contribution, find its complete
parent vector, traverse ancestry with missing-parent/cycle rejection, and compare
the reachable physical IDs to exact D1 piece IDs selected by independently
authored raw half-open ranges. Every physical ancestor must be present in its
root's ordered inventory and have that exact root as its sole direct parent.
Verify six raw coordinates, source digest and edition; unrelated equal-spelling
tokens, trivia, EOF and other input roots must not become operand ancestors.
Independently verify the ordered inventory's piece/token ordinals, closed
descriptor/counts, contiguous coordinates and unique terminal EOF. Compare the
union of every root inventory with a whole-graph physical census that includes
namespace-free legacy lexer_token/lexer_trivia rows. A path-only remint check is
insufficient. Synthetic reversed-inventory and extra unmarked-token controls
must fail the oracle itself before its actual lineage results are accepted.

Cover repeated literal spellings, binary/unary/string-length/nested folds,
equal conditional branches retaining both histories, comments/CRLF, multiline
ASI and two inputs with identical text but disjoint roots/IDs. Exact raw ranges
include quotes and astral widths; processed token lengths remain edition-specific
and cannot serve as spans. Compare traced/untraced JS/map/manifest bytes and
repeat complete graph/journal serialization for determinism. Checked reload is
still generic graph validation, not source-domain certification.

The retained cv_fold_trace operand helper currently matches the second tokenizer's
Origin.source==path and line:column. Migrate it to source-inventory lexer_token
origins, require the path-prefixed exact legacy location, exact input descriptor
and D1 inventory membership, and retain all existing expected operand sets and
nested/intermediate fold checks. The CLOC27 presence-only smoke test must likewise
stop demanding rootless parser-token origins and require the exact physical
inventory. It remains a smoke test, not a replacement for graph assertions.
Retain and run old failure/limits, stage chronology and trace-neutrality tests.
Refusal cells require exact normal exit1, empty stdout, typed Parse-stage
classification and the expected lexical/parser cause, with no panic/abort.
An arbitrary nonzero code and unchanged sentinels are insufficient. Exercise
both SIMPLE and ADVANCED at the private missing/lexical capture boundary.

After implementation run affected standalone compiler tests and strict all-target
lint, exact-source independent security, docs and required hosted gates before
merge. Do not infer Java-oracle parity from Rust test counts. Before each push,
review exact final source/docs/publisher and record Miri DEFERRED/PENDING in PR
validation. Auto-merge requires current conflict-free, review-clean, terminal
acceptable exact-head checks; then verify actual merge reachability and clean
only merged idle worktrees. Current PR conflicts remain a delivery gate.

## Explicitly unfinished contract

D2 does not certify untrusted serialized source-domain imports, stamp composite
or synthetic identities, add all fold/rename/inline/motion/merge/delete lineage,
join emitted bytes to source mappings or prove full Closure parity. Stable
occurrences and complete pass/sweep events remain required. After implementation
delivery complete genuine final-source Miri, rerunning invalidated receipts.
Full objective and heartbeat remain ACTIVE until that evidence exists.

## Exact-source terminal verification

Actual traced SIMPLE/ADVANCED compilation now consumes the D1 captured Tokens

and moves their already-precharged ingestion-ID strings into Token.cv before

the existing ASI/bridge. Folds reach the exact physical input pieces and root;

no second parser-token origin set is minted. All non-cv Token fields, ID-buffer

and Tokens-vector identities are preserved. Private length/stamp/compact-ID

refusals precede mutation; missing/lexical capture cannot select legacy relex.

The public source-based transform facade retains detached-ID compatibility.

Shared parser/ASI/bridge APIs, grammar, depth/retry/production limits, dependencies,

passes and stage order are unchanged. Synthetic ASI semicolons keep cv=None;

they acquire no invented physical source pieces.



Exact source `60e58439764959178584906e4f26b148da96ae8c` passed strict all-target

Clippy and the complete unfiltered standalone compiler suite:1,112P/0F/0I across

181 metadata-derived targets. Four private ownership/refusal tests and eight D2

CLI tests pass; ten lineage cells check exact raw byte/scalar/UTF16 spans, roots,

ordered inventories and operand/merge/intermediate identities, ten baseline

cells preserve trace-off JS/map/manifest bytes and repeated graph/journal bytes,

and four failure cells require normal exit1 and the exact typed Parse cause while

preserving all destinations. Both typed levels, EOF/right-brace/restricted-return

ASI, repeated spelling, nested/string/unary folds, astral/CRLF coordinates and

disjoint identical inputs are covered. Existing eight CLOC31 tests, CLOC27 smoke,

ten D1 inventory tests and all remaining compiler targets pass; focused counts

are included in1,112, not added. Private move tests cover14 language editions.

Independent exact-source, static test-quality and terminal security reviews PASS.



The original770cabaf compiled semantic red (1P/5F) is retained, with legacy

success/byte/determinism guards and four refusal cells. The8783 test-quality

review withheld inventory-census and error-domain claims; spec/lesson/test

corrections added full legacy/reserved census, ordered ordinals/counts/EOF,

synthetic oracle negatives and exact refusal classification at both levels.

The earlier b802 focus remains historical; final60e was rerun. Helper shell-quote

and old-Python startup failures remain separate; the same source-transfer helper

completed under explicit py -3.13. No production/schema/limit weakening occurred.



Complete native2868/style872/SwiftUI191 maps remain exact to published D1a604,

retaining prior execution attribution. Closure36/3710 differs in exactly six

Rust paths: two private production files, two new test files and two migrated

legacy tests. Raw Rust/TOML1499,22 unique static native/Closure include paths and

the standalone lock/15 previously accepted registry tuples are verified. No new

native/style/SwiftUI execution or Java-oracle parity is inferred.



Source-domain import certification, composite/synthetic stable identities, full

fold/rename/inline/motion/merge/delete lineage, complete pass/sweep events,

output-byte/source joins and full Closure parity remain OPEN. Base ESTree

canonical serialized boundary, own types and no new crates.io dependencies

remain owner decisions. Sole PR17288 remains unmerged; current-main conflict

repair, required exact-head hosted checks and actual merge/reachability remain

gates. Miri **DEFERRED/PENDING**, NEVER passed; frozen checkout/run/inputs/target/

receipts untouched and unpolled. Implementation/delivery first, genuine

final-source Miri afterward; full objective and heartbeat remain ACTIVE.
