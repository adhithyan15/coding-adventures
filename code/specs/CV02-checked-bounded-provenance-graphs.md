# CV02 — Checked and bounded provenance graphs

## Scope and verified predecessor

This implements the graph and serialization foundation of CCR-065 #16868.
CV01 merged in #16881 at `ec1c8d8c733f188c762e1377fe56dc07ecd2faa7`.
All 34 checks were terminal/acceptable on reviewed head
`76f7dc78995dc6c66dfcc1f1952bea0b37efcc21`, including all three operating
systems and required gates. Fetched-main reachability and all 14 changed-file
contents were verified; the compiled baseline and audits were preserved and
the clean predecessor worktree was archived. This selection starts from
`059acbf9e06ff7c75b1c84f278d09a13adaf53ff`, with only unrelated ALGOL changes
after that verified merge. Preserve one active Closure/shared-stack PR.
Use repository libraries and the standard library; introduce no crates.io
dependencies. Preserve the owner decisions: own AST, canonical base ESTree
serialized boundary and the Babel converter.

CV01 provides constant-size identities and reliable allocator reload state.
It deliberately does not establish graph validity. Direct-library evidence:

- A -> B -> C; M=merge(A,C) returns B,C,A,M, placing a parent after a child.
- Cycles and dangling parents serialize and reload successfully.
- A fresh compact log deriving from its future first ID creates a self-cycle.
- Public mutable entry maps bypass any cached graph/accounting invariant.
- Default JSON varies across fresh processes; pretty JSON and NDJSON were stable
  in the recorded five-process SIMPLE/ADVANCED probes.
- Formatter/summary failures can return empty objects, fallback formats or zero
  counts after JavaScript/source-map/manifest artifacts have already been written.

Full compiler completion remains the backlog's five-level delivery contract.
Typed transformation chronology, lexical/composite ownership, every mutating
pass, output-byte/source-map joins and remaining folds (#16875) remain required
after this foundation. CV02 must not close CCR-065 or the compiler umbrella.

## Ownership and compatibility

1. Checked compiler logs own their graph and allocation/accounting state.
   External code must not mutate entries, parent lists, pass order or recording
   mode through public fields. Use read-only accessors and controlled methods.
   A deliberate field-access migration is allowed: update all repository callers
   and documentation, retaining default constructors, hierarchical ID spelling,
   ordinary snapshot shape and supported enabled/disabled semantics.
2. Introduce explicit checked compact construction and checked import, with a
   public limits value. Checked full recording remains enabled throughout a run.
   Generic compact/legacy allocation may still issue unstored IDs while disabled;
   that allocator-state reload contract cannot imply complete recorded history.
   Checked import must reject incomplete storage, missing allocation coverage,
   disabled recording and declared projections as full graph evidence.
3. Preserve old signatures deliberately. Add fallible paths where needed,
   including deletion. Infallible compatibility wrappers may fail fast; compiler
   code must use propagated errors and useful diagnostics. No invalid substitute
   identity, ignored mutation failure or successful incomplete trace is allowed.
   A visitor that cannot return an error may retain its first error and return it
   before accepting the transformed program; it must never publish that result.

## Limits and mutation transactions

Initial API names: `GraphLimits` exposes `max_nodes`, `max_edges`, `max_events`,
`max_metadata_values`, `max_metadata_depth`, `max_metadata_bytes`,
`max_input_bytes`, `max_work` and `max_output_bytes` with checked defaults.
`CVLog::new_checked_compact(limits)` returns a fallible enabled checked log;
`CVLog::from_checked_json(s, limits)` requires complete checked recording.
`try_ancestors`, `try_descendants` and `try_lineage` are fallible queries;
`try_lineage` returns borrowed entries. `entries()`, `pass_order()` and
`is_enabled()` expose read-only state. Existing allocation `try_*` methods and
`contribute` enforce checked policy on a checked log; add `try_delete` and
`try_passthrough` for fallible compiler use. Final code must propagate each
error to its compiler/pass/lexer/parser boundary, rather than leaving `let _`
around a newly fallible operation. Snapshot and query errors may retain their
existing string error representation if diagnostics remain clear and useful.
When a recursive pass uses a large-stack worker, reject and destroy its candidate
AST inside that worker on a provenance error. Do not return a deep rejected
candidate to a smaller caller stack only to drop it while propagating the error.

Limits cover retained nodes, parent edges including repeated edges, contribution
and deletion events, metadata JSON values/depth and bytes, input bytes,
operation work and serialized output bytes. Every addition uses checked integer
arithmetic. Rejection leaves graph, allocation watermark and retained usage
unchanged. Account before inserting or extending; never truncate to fit.

Owned mutation arguments may already exceed limits when the caller transfers
them. Rejection must dispose of nested metadata iteratively, including early
identity/parent/event failures and disabled compatibility no-ops. A work budget
bounds graph processing and serialization, not deallocation of memory already
constructed and transferred by a caller: safe disposal necessarily visits that
owned payload. Cleanup must not recurse through arbitrary caller nesting or
leak rejected evidence. Heap iterator frames may grow with caller nesting;
do not duplicate all siblings into another unbounded pending-value buffer.
Expose `dispose_metadata(HashMap<String, Value>)` for callers that own pending
evidence batches. The scheduler transfers returned event metadata into checked
recording without cloning before validation. On the first recording failure,
dispose of every remaining event's metadata iteratively before returning the
error; also dispose safely when there is no program identity to attach it to.
Preserve public record types' move-field compatibility rather than adding Drop
implementations that prevent callers from transferring their fields.

Initial checked defaults are 1,000,000 nodes; 4,000,000 parent edges;
4,000,000 contribution/deletion events; 1,000,000 metadata JSON values;
64 metadata nesting levels; 128 MiB retained metadata/text bytes; 128 MiB
import bytes; 64,000,000 structural work units per operation; and 512 MiB
serialized export bytes. Count origin/source/location/timestamp, stage/tag/
reason and metadata keys/string/scalar/container encodings in retained payload
accounting; identity/edge storage is separately bounded by fixed IDs and counts.
Structural work counts graph/JSON visits and queue/index operations; byte caps
bound string scanning/encoding, and sorted traversal must retain documented
O((V+E) log V) or better graph complexity.

Import and export share one work allowance across their phases. Bounded parse,
typed conversion, independent graph validation and encoding cannot each restart
the operation's budget. Typed conversion may conservatively reserve another
allowance equal to the parser's charged structural visits before conversion;
this includes object keys and wrappers as well as payload values.

These are explicit finite limits, not a claim of a precise RAM cap.
Callers can select other finite limits;
compiler configuration must expose a documented way to exercise or raise them.
Verification must measure real corpus usage before confirming default adequacy.

Depth has a hard serialization ceiling of 64 metadata levels; selecting a larger
depth is a configuration error, even when other finite budgets are raised.
Checked snapshot objects require the declared record fields explicitly, reject
unknown fields and validate nested duplicate keys before typed conversion.
Only null origins/tombstones/timestamps are nullable; missing parent/history
arrays cannot silently become synthetic roots or empty history. Generic
`from_json_string` remains an allocator-state compatibility import; it is not
checked graph evidence. Compiler query/import/export paths use checked APIs.

Checked roots/derivations/merges reject identity exhaustion/collision, missing
parents, self-parentage and any parent incompatible with allocation chronology.
All compact parents precede their child's allocation. Keep parent-list order
and duplicates as data, while handling repeated edges correctly in traversal.
Contributions/deletion reject unknown identities; deleted entities retain their
records and cannot acquire new history. Repeated deletion has a documented
non-overwriting policy. Checked mode cannot be disabled mid-run. Default mode's
existing disabled allocation behavior remains separately documented/tested.

## Checked import and queries

Reject excessive input before parsing. Parsing must have explicit work/depth/
payload limits and reject duplicate decoded object keys before reading a
replacement value, including nested metadata and escaped spellings. Do not
first parse through a representation that already discards duplicates.
Validate the entire graph: matching keys/IDs, correct identity version/state,
complete checked allocation coverage, resolved parents, acyclicity and limits.
Build or expose a log only after validation succeeds. Unchecked allocator-only
compatibility import, if retained, must be named/documented without claiming
validated provenance and cannot be used for compiler query/export boundaries.

Strict query validation applies regardless of construction/import policy:
enabled recording, complete compact allocation coverage and a stage declaration
set matching all recorded contributions and tombstones. Historical allocator
snapshots whose deletion stage was never declared may reload/export generically
but cannot claim complete query evidence. Generic import must reject any
declared `view`, including legacy IDs and null markers, so it cannot discard a
projection declaration and later upgrade that projection into full evidence.
Generic allocator imports must remain identifiable as unchecked evidence:
their typed conversion can discard duplicate metadata keys or synthesize omitted
arrays before later graph validation. Fallible evidence queries cannot upgrade
those imports; use the bounded checked import to establish complete evidence.

Fallible graph queries distinguish an unknown ID, an invalid/incomplete graph
and an exhausted work/output limit from an empty successful result. Implement
iterative traversal so a deep chain cannot overflow the call stack. For lineage,
include each reachable ancestor once and order every parent before its child.
Use deterministic ready selection on shared/uneven DAGs; nearest-first ancestry
and descendant queries must also be deterministic under their documented order.
Do not reverse BFS and call it topological order. Avoid rescanning all ancestry
once per descendant. Return borrowed evidence or bounded copies; cloned payloads
must not evade limits. Topology is not actual pass/sweep execution chronology.

## Serialization and artifact publication

Validate the full checked graph before filtering, summary or export. Canonical
compact/pretty JSON and NDJSON sort object keys at every graph/metadata level,
preserving array order and numeric values. NDJSON retains all root metadata and
declares projections accurately. Enforce output bytes as they are produced;
oversized bodies must never materialize an unbounded successful buffer.

Every formatter/summary boundary returns errors; no `{}`, zero-count summary,
skipped entry, changed-format fallback or apparently complete truncated document.
NONE enforces graph validity and retained/work budgets while intentionally
producing no sidecar. Serialized-byte caps apply to actually materialized
exports/summaries. Summary-only retains its documented artifact suppression.

Complete provenance validation and prepare all requested payloads before
publishing JavaScript, source maps, manifests, sidecars or successful stdout.
A provenance/serialization error must preserve pre-existing destination files
and leave no new success artifacts. Define and test filesystem publication
failure separately: stage owned temporary files, reject colliding destinations,
preserve/restore existing outputs on failure, clean only owned temporary paths
and report rollback failures explicitly. Never erase unrelated user files or
claim a successfully written event for a failed publication. Artifact linking
must identify the published content, not merely a pathname that can hold stale
bytes. Keep tracing-neutral output assertions when all operations succeed.

## Required verification

Commit specification refinements before implementation, then demonstrate the
current reproductions failing meaningful acceptance tests before repair.
Exercise compact and legacy contracts, enabled/disabled allocations, full/partial
reloads, counter and every resource boundary at/over cap, arithmetic overflow,
unknown/self parents, cycles, duplicate keys and malformed metadata. Assert
failure preserves allocator/graph/usage state and existing artifacts.

Use mixed/shared/uneven-depth DAGs, repeated parent edges, deep direct-library
chains of at least 4096 and wide graphs. Assert parent-before-child order and
bounded work/retained copies; measure scaling rather than relying on parser
depth guards or tiny CLI fixtures. Query unknown IDs and deliberately invalid
compatibility graphs; invalid evidence must not become a success-shaped result.

Run fresh SIMPLE/ADVANCED processes on identical paths/configuration for every
format, pretty/filter/summary/NONE/summary-only combination. Assert exact export
bytes, resolved full graphs, declared partial coverage and unchanged emitted
JavaScript. Inject limit/publication failures with existing destination files
and stdout output; verify error status, preserved contents and owned-temp cleanup.
Run the full affected library/compiler/consumer suites and strict lint, lessons
and whitespace checks, independent exact-head security review, actual required
cross-platform CI and fetched-main merge verification. Record remaining gaps
and keep the full compiler/provenance goal active.
