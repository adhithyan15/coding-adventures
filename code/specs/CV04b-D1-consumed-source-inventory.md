# CV04b D1 — serialize the actual consumed source inventory

## State and bounded delivery

This specification precedes tests and implementation. It builds on native18
source153516b5 in a separate managed worktree and a separate planned Cargo
target, while R2 native18 full producer3984 remains frozen and untouched. Keep
one remote implementation PR, currently17288, and deliver native18 with its own
terminal receipts before publishing this successor. No new implementation PR.
This directly advances remaining Closure source provenance; incoming unrelated
main changes remain audited pending work, not implicitly accepted here.

Owner decisions remain base ESTree as the canonical serialized AST boundary,
owned Rust types, and no new crates.io dependencies. This slice changes only
compiler-private ingestion recording and its tests/docs. Shared lexer, parser,
AST, graph APIs, limits and native inputs remain unchanged. Existing public
transform_source_with_cv and all standalone parser/tokenizer entrypoints retain
their signatures and behavior. CV tracing off uses the current tokenizer path.

The current four-file leaf audit at153516b5 and main81164a49 proves CLOC27 leaf
CV propagation and CLOC31 checked primitive derive/merge are real. The compiler
still separately mints ingestion child IDs and calls the typed CV parser, which
retokenizes and mints independent leaf IDs. This corrects the epic's old allNone
bridge assessment. D1 records raw ingestion pieces and finalized root binding.
Actual captured-ID reuse by ASI/bridge, source-domain import admission, composite
identities, all later transform lineage, emitted-byte joins and complete parity
remain follow-ups; do not label them completed by a valid D1 graph.

## Actual capture and failure ownership

With tracing enabled, call the existing standard JavaScript grammar capture
API tokenize_javascript_typed_with_ranges on the exact read_to_string UTF8
buffer before creating the input root or reserved metadata. Use the configured
EsVersion and unchanged LexicalLimits defaults/ceilings. Its private successful
LexicalSnapshot is the actual matcher proof; a caller-built partition is not.
Read descriptors while borrowed, then consume into_tokens without retokenizing
or cloning the entire descriptors/source. Preserve all token fields, processed
values, legacy locations, type_name and newline/ASI flags. Continue the existing
ingestion token contributions/defines and subsequent compilation behavior.
D1 does not route transformed define strings through stale input ordinals.

Only LexicalError::Lex may become an unavailable inventory with the existing
lex.failed behavior. Unsupported, Limit, Invalid, Allocation and graph/recording
errors are provenance failures and fail before publication of any output set.
SIMPLE/ADVANCED parse failure remains failure; unavailable never authorizes a
lower compilation-level fallback. BUNDLE/TRANSPILE_ONLY/WHITESPACE_ONLY retain
ordinary lexical-error behavior with an explicit unavailable inventory.

Capture/metadata failure may leave an unpublished internal CV log; no finalized
inventory or partial output set may be published. Preserve existing sentinel
JS/map/manifest/trace files on all recording/graph-limit failures. Tracing-off
valid/invalid-source behavior remains unchanged.

## Closed producer namespace

Reserve metadata key closurec_source_v1. Its value is a closed JSON object with
literal schema "1" and role "input", "piece", or "inventory". Unknown/default
fields are not emitted. Every numeric field is a canonical decimal String:
"0" or a nonzero digit followed by digits, checked u64/usize conversion, at
most20 digits; never JSON numbers, signs, padding, whitespace or exponent forms.
This is producer schema only. Generic checked JSON graph import is still
required but is not source-domain certification; the replay/duplicate-aware
source admission API belongs to a subsequent bounded slice.

Input Origin remains source=input_file, unchanged display-path location and
None timestamp. Its reserved role=input descriptor has exactly schema, role,
buffer_domain="read_to_string_utf8", encoding="utf8",
grammar="javascript_standard", edition=EsVersion::as_str(), sha256=64 lowercase
hex digits of the exact buffer, and byte_length. Existing byte_len numeric and
content_sha256 legacy metadata remain and must agree. No normalization or
prior decoding adapter is certified by this domain.

Every actual LexicalPiece gets exactly one direct child of this input root,
in complete consumed-piece order. The reserved role=piece object has exactly
schema, role, input_id, piece_index, kind, start, end. Kind is token/trivia/eof.
Token and EOF additionally have token_index and token_type; trivia has neither.
Each start/end object has exactly byte_offset, scalar_offset, utf16_offset,
line, scalar_column, utf16_column, with CV04's zero-based CRLF/bare-CR/LS/PS
semantics and half-open UTF8 intervals. EOF is unique, terminal and zero-width.
No raw substring or processed token value is copied into this namespace.

Token/EOF Origin.source remains lexer_token, with unchanged legacy token line/
column display location and existing kind, token_index, lexeme_byte_len fields.
The latter remains processed value length and is explicitly not consumed size.
Trivia Origin.source is lexer_trivia, location uses the raw start line/scalar
column plus1, and has only its reserved descriptor. Every piece input_id and
its actual parent vector must identify the exact file root; all allocated
piece IDs must be distinct, including repeated equal lexemes and EOF.

Freeze token_type through an exhaustive match over the shared enum:
Name, Number, String, Keyword, Plus, Minus, Star, Slash, Equals, EqualsEquals,
LParen, RParen, Comma, Colon, Semicolon, LBrace, RBrace, LBracket, RBracket,
Dot, Bang, Newline, Indent, Dedent retain exactly those spellings; Eof -> EOF.
This names enum variants, not an assertion that every variant occurs in the
standard JavaScript grammar. Captured type_name/flags remain in actual Tokens.
Do not substitute Debug/Display as an extensible wire mapping.

After every piece is recorded, append exactly one input-root Contribution with
source="source.inventory", tag="finalized" and the reserved role=inventory.
Its exact fields are schema, role, status, piece_count, token_count, piece_ids.
Available status uses the complete ordered ID array, including trivia and EOF;
token_count includes EOF and agrees with the original captured stream. An
unavailable inventory has both counts "0", empty piece_ids and the sole extra
reason="lexical_error" field; available forbids reason. No available partial
list, extra/missing/repeated child or legacy-only success may escape.

Only IDs minted by this run's private checked compact allocator are used here:
cv1. plus16 lowercase hex digits. Check shape before reference ownership. Do
not infer bounds for arbitrary imported IDs; imported-domain acceptance is OPEN.
The existing token_cv_ids vector must select exactly token/EOF entries in token
order from this inventory, preserving define/whitespace contribution ownership.

## Deterministic precharge and finite ownership

Use one private per-run SourceInventoryLimits/budget for all newly owned
reserved values, encoded bytes, additional reference storage and work. Defaults:
1000000 values,64MiB encoded metadata,16MiB reference storage,64000000 work.
Configuration ceilings:4000000 values,128MiB metadata,64MiB references,512000000
work. Zero limits are legal refusal controls. Reject overflow/ceiling errors
before recording or new bounded ownership. Existing graph and lexical limits
remain independent and unchanged; either may reject first.

Precharge the complete prospective file descriptors/inventory before building
any serde_json Value, decimal String, metadata HashMap or inventory ID copy.
Use arithmetic decimal lengths and known compact-ID/hash lengths; do not build
an unbounded temporary Value merely to measure it. Count namespace wrapper,
role/coordinate objects and scalar/array nodes. Closed-schema value counts are:
input10; trivia21; token/EOF23; available inventory8+n IDs; unavailable9.
An available file therefore costs18+22*trivia+24*token_or_eof reserved values.
An empty valid source has one EOF and costs42; an unavailable file costs19.
Legacy origin/contribution metadata and graph containers are independently
charged by the unchanged generic checker, not silently included in this table.

Encoded bytes use compact serde string semantics: quoted J(s) counts UTF8 bytes,
escapes of quotes/backslashes/short controls as2, other controls as6. Objects cost
2+max(field_count-1,0)+sum(J(key)+1+B(value)); arrays cost
2+max(item_count-1,0)+sum(B(item)); strings costJ(s). Charge namespace wrapper,
keys and all decimal/reference strings. Compare the arithmetic independently
against actual compact serialization for every descriptor variant.

Additional ID/reference vectors and String copies use checked size arithmetic
and fallible reservations. Precharge requested growth before ownership, count
actual backing capacity and transient/retained copies, and reject allocator
capacity exceeding the admitted bound. Do not claim the bound covers generic
CVLog ownership, existing token values or allocator bookkeeping. Small exact/
one-less tests must demonstrate value/byte/reference rejection without large
allocations. Serde maps lack a fallible insertion API; bounded prospective
ownership is required, but recovering every system allocator OOM is not claimed.

Work charges source bytes, visited pieces, reserved metadata values and compact
encoded bytes, plus each new reference occurrence. Charge before each bounded
scan/ownership step using checked arithmetic; no per-piece full-graph scan or
source clone. Existing regex work is outside the additional descriptor budget
and keeps its existing limits. Do not promise all262144 default lexical pieces
fit generic graph's1000000 metadata-value default.

## Evidence and delivery

Tests commit first and fail semantically against the existing producer: verify
exact whole reserved objects, full ordered inventory, actual direct parent IDs,
all source coordinates, raw versus processed lengths, distinct repeated-token
identities, one terminal EOF and no trivia token ordinals. Hand-author complete
ASCII/repeated/empty/comment/trailing trivia and Unicode/escaped-string/CRLF/
bare-CR/LS/PS tables. Test every supported edition against unchanged Tokens.
Retain existing graph, chronology, pass-order, lexical/native limit and ABI tests.

Run actual native CLI subprocesses with tracing on/off at all five levels and
compare JS/map/manifest bytes and exact exits. Repeated traced runs must preserve
identical complete graph/journal output. Test multiple files, defines and source
errors, metadata/graph budget refusal and failed-publication sentinel preservation.
Do not replace lineage checks with token/field presence, and do not claim the
old typed-parser leaf IDs are already the actual ingestion IDs in this slice.

Run affected compiler tests and strict all-target lint, checked graph import/
roundtrip controls and required downstream checks in a separate target. Derive
complete native/style/Closure/source input maps and exact deltas for delivery;
R2 native producer/source/lock and frozen Miri remain untouched. Independent
exact-head source and terminal security and required hosted checks remain gates.
No merge/auto-merge without current conflict-free review-clean exact-head terminal
acceptable required checks and actual merge reachability. Preserve unmerged
worktrees and unrelated edits; no force push or new parallel implementation PR.

Miri DEFERRED/PENDING, never passed. Implementation/delivery first, genuine
final-source Miri afterward. This spec, D1 implementation, and even passing D1
producer tests do not complete the full Closure parity/lineage contract or justify
disabling the heartbeat.

## First test-fixture classification correction

The first D1 test-only run4afe02af actually returned1P7F: six exact input
descriptor failures and one invalid lexical-error control. The legacy/capture
baseline guard correctly refused classifying @ as an ordinary ES2025 lexer
error, because that edition adds an AT decorator terminal. Preserve its full
log/receipt and wrapper1; do not claim seven producer-semantic failures. Change
only the error fixture to the existing CV04-tested var x= followed by NUL,
retain @ as an available ES2025 companion, and test its earlier-edition refusal.
Keep all exact inventory/schema/parent/coordinate assertions and source APIs.
