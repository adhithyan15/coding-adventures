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
chronology never become stronger evidence through a shape check. Raw untrusted
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
Compact IDs are canonical and densely covered after generic graph validation;
a borrowed inventory-pointer vector indexed by compact sequence allows O(1)
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
