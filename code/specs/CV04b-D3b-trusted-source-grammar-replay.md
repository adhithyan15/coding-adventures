# CV04b D3b — bind consumed buffers and replay the standard grammar

This specification precedes tests and implementation. D3a is published in the
sole active PR17288 at6dcda953, with1,124P/0F/0I across182 compiler targets,
strict lint and independent reviews. It admits SHAPE, not source content.
The current audit re-fetched main4014a701, all13 open PR file sets, both backlogs
and provenance epic15830. PR17288 is OPEN/CONFLICTING/autoOFF/0 attached checks.
No hosted/current-main/merge acceptance is inferred. Preserve unrelated edits
and the frozen Miri run/checkout/inputs/target/receipts, untouched and unpolled.
Miri **DEFERRED/PENDING**, never passed; full objective/heartbeat ACTIVE.

## Audited boundary and scope

D1 already captures the actual read_to_string UTF8 buffer using the standard
edition-specific JavaScript matcher, including trivia, six raw coordinates and
EOF. D2 moves its exact captured Tokens/IDs through typed ingestion; CLOC27 leaf
reuse and folding are present. Do not recreate them or repeat the epic's stale
bridge claim. The raw checked CV importer ALREADY rejects decoded duplicate keys
at every object depth; reuse it on original text in untrusted-snapshot tests.

D3a validates generic graph evidence, dense compact IDs, closed metadata and
the independent physical census. Its hash is only self-consistent text. Raw
coordinates may be structurally contiguous yet false; token/trivia kinds and
legacy value lengths/positions may be plausible yet disagree with the grammar.
The public detached lexical-partition validator explicitly does not recognize
grammar or authenticate source. It cannot supply D3b evidence.

The compiler currently drops each actual input String and its snapshot pieces
before final publication. A later filesystem read would establish different
authority if a file changed. D3b therefore explicitly retains the ACTUAL consumed
String allocation, moved into a finite compiler-private catalog. It also stores
bounded fingerprints of every original captured Token before defines/stamping/
ASI mutate or consume that stream. No source, Token, metadata, path or ID strings
are cloned for catalog ownership. Per-file replay uses the original consumed
bytes and actual configured edition, not descriptors' claimed values.

Only compiler-private modules and tests change. Reuse the existing local SHA256
and JavaScript lexer; no new crates.io dependencies, shared-crate API, public
parser/transform signature or CLI/config flag. Base ESTree canonical serialized
boundary and own types remain owner decisions. No grammar, pass/sweep order or
existing graph/lexer production limit is changed. Tracing off is unchanged.
Valid JS/maps/manifests/CV graph/journal/summary bytes remain identical to D3a.

## Trusted ownership and admission sequence

Create the catalog only for traced compilation, using the resolved input count
and actual configured EsVersion. Precharge and fallibly reserve all slots before
owning that index. For each file, record D1 capture from the actual String as
today, then retain that SAME String by move and borrow its str for all existing
per-file transforms. Charge actual String capacity, not merely len. Reserve and
build the original-Token fingerprint vector before committing the catalog slot;
failure leaves the prior catalog prefix/ledger unchanged and publishes nothing.
Return a borrowed str into the retained allocation, preserving its pointer and
capacity. This avoids a two-phase pending-proof protocol or unbounded clones.

The catalog stores the compact root sequence, moved source, and optional ordered
Token fingerprint vector. Some means successful original capture (including
EOF); None means D1's ordinary Lex error only. Initial Tokens must have cv=None;
fingerprint before defines/ID stamping/ASI. Roots must be canonical compact,
nonzero and strictly increasing in actual input order; retain exactly one slot
per resolved input, with no implicit Vec growth after reservation.

At the existing prepublication boundary, D3a runs first and yields a private
borrowed ShapeAdmitted wrapper with an inaccessible constructor and read-only
log access. This wrapper proves ONLY the already checked graph/shape premise;
it is not source certification and is never serialized. The production caller
then passes it, the catalog, actual resolved paths and actual per-file root IDs
to D3b. Existing String-returning D3a unit facades stay test-only. Keep private
Graph/Shape error classification and existing validate/snapshot diagnostics;
new catalog/replay refusals become CompilerError::Provenance at source_replay.

Require exact catalog/path/root cardinality and ordered root-sequence agreement.
Independently census ALL input_file roots in the admitted full graph and require
that count to equal the unique trusted roots. This catches omitted/extra sources,
even empty inputs, before filters can hide them. Match each root's actual path
representation, declared supported edition, byte length and digest against its
trusted consumed String and configured edition. Do not trust descriptor text as
the grammar selector. Never read an input file during this validation.

All graph and presentation choices use the same full replay gate: JSON, NDJSON,
NONE, filtered/unfiltered and summary routes. Gate before any artifact/summary
publication. Preserve existing no-emission diagnostic/checks-only paths; they do
not acquire a claim about emitted provenance. After replay, compiler code only
reads the graph for encoding/summary/publication; no later graph mutation is
permitted before publishing that artifact set.

## Replay and exact token evidence

Replay one retained input at a time with the existing
tokenize_javascript_typed_with_ranges and LexicalLimits::default(). Keep at most
one replay snapshot live, and discard it before the next file. Existing lexical
source/piece/retained/work bounds and their independent regex-cost exclusion
remain in force. Only LexicalError::Lex may justify unavailable. Unsupported,
Limit, Invalid and Allocation errors refuse admission; never convert them to an
unavailable success or retry a weaker tokenizer.

An available declared inventory must agree with actual replay success and
original successful-capture status. Require exact physical piece count, token
count, piece kind, token ordinal/type, and all six start/end coordinates for
EVERY piece including trivia and terminal EOF. Traverse each root inventory's
contributions once and each listed piece once; never rescan roots per piece.
D3a's fixed ordered inventory/sole-parent/census proof remains the precondition.
Compare legacy token kind/index, processed value.len and exact processed
line/column location with replayed Token data. For trivia use the producer's
raw line+1/scalar-column+1 convention. Do not infer processed locations/lengths
from raw spans: bare CR, CRLF, astral text and escaped/quoted strings differ.

Each reconstructed Token must additionally match its retained original-Token
fingerprint in order. The fingerprint is SHA256 over a fixed domain separator
the exact bytes b"closurec.lexical-token.v1\0" followed by unambiguous
length/option framing: frozen
TokenType wire name, full value UTF8 bytes, checked u64 little-endian line and
column, type_name None/Some plus length-prefixed bytes, flags None/Some plus
u32 little endian, and an explicit cv=None tag. Encode string byte lengths as
checked u64 little endian. Distinguish None from Some(0)/Some(""). No Debug,
serde ordering, concatenated scratch string, or ambiguous delimiter encoding.
This commits full original Token fields; checking only value length is weaker.
It does not certify later transformed Token/AST/output-byte lineage.
Exhaustively destructure Token without a rest pattern, and reuse the producer's
exhaustive TokenType spelling match, so future fields/variants cannot silently
escape the full-Token commitment. These clarify the reviewed42b9 contract before
tests and implementation; its contract-only PASS is retained separately.

Use existing Sha256Hasher in chunks of at most64 bytes: its update currently
buffers/drains input, so handing it an entire source/value would copy data and
make repeated drains quadratic. Bounded chunks keep its scratch bounded;
digest is a fixed32-byte array. Decode the already shape-checked lowercase
64-character descriptor hash to compare fixed digest bytes, without allocating
a source-sized padded buffer or a second owned hex String.

An unavailable declaration must agree with actual ordinary Lex failure AND
original unavailable status, and retain no token fingerprints. D3a already
requires zero counts, no physical pieces and reason=lexical_error. Confirm that
declaration against the trusted source; its text alone is insufficient.

## Finite resource contract

Private compilation-wide defaults: retained source CAPACITY64MiB, catalog index
16MiB, Token fingerprints16MiB, work64,000,000. Hard ceilings respectively:
256MiB/64MiB/64MiB/512,000,000. Validate configuration before allocation or work.
All products/sums and usize/u64 conversions checked. Prospective full slot bytes
are expected_inputs * size_of::<TrustedInput>(); fingerprint bytes are
token_count * size_of::<[u8;32]>(). Precharge before fallible exact reserve, check
actual capacity against admission, and never permit implicit growth. Source
String was already allocated by the original read; this budget governs its
additional retention, not that pre-existing allocation.

Charge every catalog/root/event/piece/Token/field visit and each bounded text,
numeric conversion and digest scan BEFORE visiting it. Prepay the complete
canonical Token encoding size before hashing that Token. Source digest input
bytes and fixed hash block visits are prepaid before streaming. Each compact
decode is charged, including final root comparisons. Charge a conservative
checked path-encoding/lossy-conversion bound before path formatting; no borrowed
metadata/ID/source is cloned to perform comparisons. Test independently derived
small-input storage/work expectations alongside exact/one-less/zero controls;
reading the implementation's own counter alone is insufficient evidence.

Freeze this conservative comparison schedule before implementation. New catalog
reservation costs one work unit per slot. Each retain costs one input visit,
then one compact-decode visit plus two20-byte scans (41); each fingerprint costs
one Token visit + twice the complete framed encoding byte length +128 prepaid
digest-finalization units. The fixed128 covers bounded final padding/scratch;
the encoding-dependent term covers every full field/hash scan, including long
and non-ASCII values. Refuse stamped Tokens before commitment.

Final replay censuses each graph entry at one visit + its Origin.source byte
length, if present. Per trusted input prepay512 units for the already D3a-bounded
root/inventory scalar fields/lookups, the41-unit compact decode, nine times the
actual path's encoded byte length before lossy conversion/comparison, each
visited contribution at1+source.len+tag.len, the source digest at source.len +
ceil(source.len/64) +128, and one capture invocation. Every actual piece prepays
1024 units for its D3a-bounded fields/IDs/coordinates and three times its legacy
location byte length before splitting/comparison; each replay Token additionally
pays the full framed fingerprint cost. Fixed root/piece charges conservatively
cover bounded numeric/text/field scans, not arbitrary ordinary metadata.

Independent examples: one unavailable NUL input, root1 and path input-0.js has
43 retain work +11 census +512 root +41 compact +90 path +26 inventory-event
+130 source-hash +1 capture =854 total. One available source "1", same path,
Number+EOF, has3110 total plus twice the independently framed original Token
fingerprint costs. These schedules exclude separate generic/shape/lexical work.
Test them literally and require exact/one-less behavior. In compiler sentinel
tests, a work threshold equal to independently derived complete retention cost
must permit retention yet refuse at FINAL replay, so deleting the final gate
cannot make the tests pass. Retain early-capacity refusal cells separately.

The ledger covers catalog ownership and replay comparison/hash work. Generic
graph, D3a shape, lexical capture and existing regex costs keep their own bounds.
Existing lexer/hasher/path formatter allocations are not claimed to offer
universal allocator-OOM recovery. No production-limit or adversarial fixture is
weakened to make new tests pass. Resource errors precede all output publication.

## Required evidence and delivery

Commit tests first. Preserve shape-only baseline controls: individually forged
content hash, edition, raw coordinates, token/trivia classification, legacy
processed length/position and available/unavailable status must pass original
checked raw import AND D3a shape, yet fail D3b replay for its intended reason.
Mutating/omitting/reordering original Token fields must fail the full fingerprint
check, even when wire metadata/processed lengths still match. Add fingerprint
framing controls for every field, None/Some and ambiguous string concatenations.
Missing-helper compilation is not semantic red; retain a compiling actual
generic/shape-only baseline and identify the narrow semantic assertion failures.

Test all14 editions and empty/comment/CRLF/bare-CR/astral/escaped/repeated tokens,
disjoint identical inputs, actual ordinary Lex failures and actual production
capture-limit refusals. Prove source/String allocation moves by pointer/capacity
identity, prior-prefix immutability on budget refusal, complete root census and
strict root-order/cardinality controls. Change a file after retaining its actual
read buffer: replay must use retained consumed bytes and perform no reread.
Resource exact/one-less/zero/ceiling/overflow checks must be meaningful.

Exercise actual compiler prepublication sentinel refusal in all five levels,
three formats and filters. Compare prior D3a and new CLI JS/map/manifest/trace/
stdout/stderr bytes using identical paths; distinguish PLAIN/NONE sentinels from
emitted traces. Run full affected standalone compiler suite and strict all-target
lint, exact-head independent source/test-quality/terminal/security review, docs
and current-main build-plan checks. Keep same sole PR and normal progress push;
merge/auto still needs current conflict-free review-clean terminal acceptable
exact-head checks and actual merge reachability before worktree cleanup.

Reconcile backlog/issues concisely. Full transformation/pass-sweep lineage,
stable composite/synthetic occurrences, output-byte joins, evidence-backed
Closure parity, current-main conflicts/required CI/merge and genuine final-source
Miri remain OPEN after this bounded source-replay slice. Do not disable the
heartbeat or claim full completion until deferred Miri genuinely passes.
