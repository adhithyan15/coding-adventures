# CV04b D3a — admit the closed source-inventory shape before publication

This contract precedes tests and implementation. Published D2 at7e1a28f5
connects actual captured input IDs to typed leaves and checked folds. Its
1,112 compiler tests/181 targets and strict lint passed; PR17288 is still
unmerged/conflicting/autoOFF. Reuse isolated R4 and its independent Cargo target.
Preserve unrelated edits and all frozen Miri inputs/run/target/receipts.
Miri **DEFERRED/PENDING**, never passed. One active Closure/shared PR.

## Audit and bounded scope

Generic CV checked import already has a bounded raw JSON decoder that rejects
duplicate decoded keys at EVERY object depth, including reserved metadata.
It also rejects views, allocator-only snapshots, missing fields and invalid
graph/journal evidence. Reuse it; do not build a second duplicate-key parser or
claim this existing protection is missing. Its ordinary metadata remains
schema-agnostic: a structurally valid graph can carry a forged source namespace.

D3a adds a compiler-private, read-only source-inventory SHAPE gate, called on
the full log before ANY traced artifact/summary publication, before filters,
for JSON, NDJSON and NONE alike. It changes no AST, lexer, shared graph API,
grammar, pass/sweep order, canonical boundary, manifest or dependency. Base
ESTree canonical serialized boundary, own types and no new crates.io dependencies
remain owner decisions. Tracing off is unchanged. Successful traces, JS, maps,
manifests and journals remain byte-identical.

This is the first part of D3, not complete source-domain certification. It
does not compare a descriptor with trusted consumed buffers or replay the
grammar. D3b must bind actual bytes/hash/edition and replay matched/trivia/EOF
classification and legacy Token data. Never expose a source-certified type,
serialize a certification marker or promote generic graph checks to that claim.
Composite/synthetic IDs, remaining transformations, output joins and parity
stay OPEN. No public parser/transform signature or CLI flag is added.

## Admission boundary

The private validator first validates its own finite limits, then requires
complete generic graph evidence through validate_graph. Thus disabled recording,
allocation gaps, allocator-only reloads, cycles, dangling parents and invalid
chronology never become stronger evidence through a shape check. Generic graph
validation permits an absent journal; D3a validates present chronology and does
not manufacture missing chronological evidence or assert full pass/sweep history.
Preserve existing generic-error diagnostics: NONE reports validate stage and
serialized formats report snapshot stage. A private typed error distinguishes
generic graph refusal from new shape/configuration refusal (source_domain stage)
without validating twice or classifying error-message text. This amendment
follows the retained full-suite6e88097d diagnostic regression before its repair;
the existing work-limit/sentinel test remains unchanged.
Raw untrusted
snapshot tests must call from_checked_json on original text before shape checks;
never normalize via serde_json::Value or the compatibility loader first.

Reserve closurec_source_v1 exclusively for input/piece Origin metadata and a
source.inventory/finalized contribution on that same input. Reject unknown
roles/locations, reserved metadata in deletions or unrelated contributions,
inventory contributions lacking the namespace, and namespace-free legacy
input_file/lexer_token/lexer_trivia entries. Ordinary unrelated metadata stays
under generic graph limits. Every input_file must have one closed input
descriptor, no parents, no timestamp, matching legacy byte_len and
content_sha256, and exactly one finalized inventory. Token/trivia origins retain
their declared legacy kind/index/length/location fields. Legacy processed
positions and lengths are NOT inferred from independent raw coordinates.

Input descriptor keys are exactly schema,role,buffer_domain,encoding,grammar,
edition,sha256,byte_length. Require D1 literal values, a supported edition and
64 lowercase hexadecimal hash characters. Piece descriptors have exactly the
seven base fields and, for token/EOF, the two token fields. Coordinate objects
have exactly the six D1 fields. Numeric strings use canonical unsigned u64
decimal, at most20 characters, checked conversion where indexing is needed.
Token type names use the existing frozen25-name producer alphabet.

An available inventory has exactly schema,role,status,piece_count,token_count,
piece_ids. An unavailable one also has reason=lexical_error, both counts zero
and an empty ID array. This only checks the unavailable declaration's shape;
proving that only an actual Lex error justifies it belongs to D3b replay.

For every available inventory require exact piece count and token count, exact
piece/token ordinals, unique IDs, direct sole root parents, matching input_id,
contiguous raw positions starting at all-zero coordinates, positive-width
token/trivia intervals and one terminal zero-width EOF. Its final byte offset
must equal the declared input length. Offsets and lines cannot move backwards;
within one line columns cannot move backwards. Cross-source reuse, duplicate
IDs, missing/extra pieces, misplaced trivia/EOF and noncanonical numbers reject.
Each physical origin in the entire graph must occur once in its exact root's
inventory; include namespace-free legacy rows in this independent census.

## Finite ownership and linear traversal

Never scan a root's entire contribution list once per piece. Use two bounded
passes: locate each root's inventory once, then validate its listed pieces.
Generic graph validation ALSO supports valid hierarchical logs: it is not a
compact/dense precondition. Before reserving/indexing, explicitly require EVERY
nonempty record key to be canonical compact-v1 lowercase16-hex format, equal
its entry ID, with sequence in1..=record_count. Unique canonical keys and that
range establish dense coverage. Reject a valid hierarchical generic log rather
than indexing it. Include hierarchical and out-of-range negative controls.
A borrowed inventory-pointer vector indexed by compact sequence then allows O(1)
root access. Charge its complete prospective bytes before fallible exact reserve,
check actual capacity, and retain no cloned metadata, IDs, sources or descriptors.
Use ordinal/parent checks plus complete physical/listed counts to detect duplicate
or omitted ownership without an additional unbounded set. All arithmetic is
checked. Charge each graph/event/piece/field and bounded numeric/text scan before
visiting it. Do not claim that this budget covers existing regex or generic
decoder work; those retain their independently enforced bounds.

Private shape defaults: index storage64MiB and work64,000,000. Hard ceilings:
index256MiB and work512,000,000. Zero/exact/one-less cases must be meaningful.
Reject invalid configuration/overflow before index ownership; allocation or
budget refusal is a provenance failure before publication. Existing generic
graph/lexical production limits stay unchanged; never reduce their adversarial
fixtures to make the new gate pass. No universal allocator-OOM recovery claim.

## Required evidence and delivery

Commit tests first. Preserve real schema-agnostic baseline controls: complete
checked graphs with individually malformed reserved fields pass GENERIC import
yet fail the new domain-shape gate. Missing-helper compilation is not semantic
red evidence. Test raw duplicate/escaped-duplicate nested namespace keys against
the EXISTING checked importer as retained positive security controls.

Exercise exact closed fields/roles/statuses/numerics, legacy agreement, root/
piece/inventory placement, unavailable declarations, ordinal/census/parent/
cross-input/EOF/coordinate cases and resource boundary/overflow refusals.
Fixtures must contain one intended violation after all earlier graph/journal
requirements pass. Preserve original malformed raw bytes separately. Include
traced CLI JSON/NDJSON/NONE/filtered-view checks and all five levels, comparing
JS/map/manifest/journal bytes to prior valid behavior; prepublication failure
must preserve every destination. Unit failure injection may exercise the private
gate; no production flag or weakened limit is permitted for tests.

Run affected compiler tests and strict all-target lint, exact-head independent
security and required hosted gates. A docs-only carrier must prove complete
source identity transfer. Use the same sole PR and verify ordinary publication;
merge/auto requires current conflict-free review-clean terminal acceptable
exact-head checks, then actual merge reachability before cleanup. Main conflict
repair and full Closure parity/source-domain replay/full lineage/output-byte
mapping remain gates. After implementation delivery, obtain genuine final-source
Miri acceptance; full objective and heartbeat stay ACTIVE until then.

## Executed bounded slice

Traced compilation now checks the full physical source inventory before any
filter, JSON/NDJSON/NONE selection, summary or artifact publication. The private
read-only gate requires complete generic graph evidence, dense compact IDs,
closed source metadata, exact ordered root/piece ownership, six-coordinate
continuity, legacy-field agreement and finite prepaid work/index storage. It
borrows metadata and allocates one fallibly reserved pointer index. Existing
generic work-limit diagnostics retain their validate/snapshot stages; new shape
refusals use source_domain. Public APIs, grammar, passes, dependencies and
production limits stay unchanged.

Exact source `c4278413686141c1b97640ae6a46a7ab85a5b759` passed strict all-target Clippy and the complete
unfiltered compiler suite:1,124P/0F/0I across182 metadata-derived targets.
Ten private tests include68 independently generic-valid schema refusals and30
actual compiler transaction refusals across five levels, three formats and
filtered/unfiltered views, preserving all four destination sentinels. Two raw
baseline tests retain the existing nested decoded-duplicate rejection. A separate
offline build of the accepted D2 source snapshot and the successor each ran35
CLI cells; all210 JS/map/manifest/trace/stdout/stderr artifacts match byte for
byte, including complete JSON graph/journal bytes. Focused counts are included
in the full suite. PLAIN/NONE trace destinations retain the prior sentinel;
they do not emit traces. Exact source, test-quality and terminal security reviews PASS.

This is structural shape admission. Trusted consumed-byte/hash/edition binding
and grammar replay remain D3b; no source-certified capability or serialized
certification marker is produced. Remaining composite/synthetic identities,
full transformation/pass-sweep lineage, output-byte joins and Closure parity
remain OPEN. Miri **DEFERRED/PENDING**, never passed; its frozen run, inputs and
receipts remain untouched. Current-main conflicts, required hosted checks and
actual merge/reachability remain delivery gates. Full objective/heartbeat ACTIVE.

Retained evidence: original69/547c/28 withheld reviews, revised39 contract PASS,
compiled generic-only red245f924 (1P/5F, three shape-mutant groups only), and
compile-failed4ab remain distinct. First full6e failed the unchanged NONE
generic diagnostic test; spec11d/testc56/private typed-error repairc427 fixed
it, and the final full suite passed. Shape/resource/extra-child controls were
strengthened after review without lowering production limits. Local source
receipt `CV04b-D3a-full-c4278413.json` SHA256 `64f2b6a1da28af91fbac05d8dd29d01fe6c13129a6c1f5bbe29856fc237b9629`;
prior-byte receipt SHA256 `87b40d4cc9f3916bae6c00078fe2c93b5b6819ca8cdea6d1217e0ddf67c1beb4`. Native2868/style872/SwiftUI191
maps preserve prior execution; Closure3713/four-Rust-file delta, raw1502/static22
and unchanged51-package lock/15registry dependencies are proven. No fresh
downstream or Java parity execution is inferred. Same sole PR17288; ordinary
progress publication still pending at this documentation checkpoint.
