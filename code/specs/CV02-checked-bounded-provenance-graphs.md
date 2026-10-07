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
Charge each object-key visit before decoding the key. Validate its role,
duplicates and encoded metadata-key budget before copying a borrowed decoded
key into owned storage. Serde's escaped-string scratch remains bounded by the
already-checked input-byte cap; the limits do not claim a precise RAM cap.
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
An allocator-only import retains a private trust marker that cannot be cleared
by recording new entries. Its canonical snapshot includes `unchecked_import:
true`; checked import rejects any presence of this field. Generic save/reload
keeps the marker and allocator behavior. Ordinary controlled-constructor
snapshots retain their existing shape; checked imports/constructors have no
unchecked marker. Serialization cannot launder normalized/defaulted evidence
into a checked snapshot.

Fallible graph queries distinguish an unknown ID, an invalid/incomplete graph
and an exhausted work/output limit from an empty successful result. Implement
iterative traversal so a deep chain cannot overflow the call stack. For lineage,
include each reachable ancestor once and order every parent before its child.
Use deterministic ready selection on shared/uneven DAGs; nearest-first ancestry
and descendant queries must also be deterministic under their documented order.
Do not reverse BFS and call it topological order. Avoid rescanning all ancestry
once per descendant. Return borrowed evidence or bounded copies; cloned payloads
must not evade limits. Topology is not actual pass/sweep execution chronology.

## Owned AST cleanup at pass boundaries

A scheduler owns both the current program and any returned candidate. Rejecting
an event, propagating a pass error, rejecting the dependency graph or replacing
an accepted intermediate program must dispose these trees without recursive
Rust drop on the caller's stack. Deep trees returned by a custom public pass
must receive the same protection as built-in folds. Metadata disposal alone
cannot establish this property.

The shared AST exposes `dispose_program(program: Program)`, which consumes the
owned tree with an iterative heap work stack. Move children and vector iterators;
do not clone, leak, serialize or spawn a fallible cleanup thread. Process one
vector element at a time, so wide sibling lists do not require another copied
buffer. Match every recursive enum variant and destructure node fields
exhaustively, making added fields/variants require a cleanup decision at compile
time. This function does not change normal ownership or wire-format semantics.
Scheduler guards apply it on error paths and intermediate replacements; transfer
only the accepted final program to the caller. Callers still own final outputs.

Verification uses an isolated 128 KiB caller, a flat input and a prebuilt
4096-wrapper candidate whose event is rejected. Also cover deep current trees
on direct pass errors and dependency-order errors, plus successful replacement.
Exercise all recursive AST families in the disposal library, including parameter
defaults, class members, module exports and nested statement containers.

## Compiler limit configuration and diagnostics

`GraphLimits::validate(&self) -> Result<(), String>` is public so CLI and
programmatic configuration can enforce the same hard ceiling before input work.
Invalid override syntax returns `ConfigError::InvalidProvenanceLimits(String)`;
its diagnostic starts with `--correlation_vector_limits:`. Provenance execution
errors use exit status 1, matching failed compilation, while existing I/O
failures retain their established status.

`SpecialModesConfig.correlation_vector_limits: GraphLimits` stores the selected
limits and defaults to `GraphLimits::default()`. `GraphLimits` supports ordinary
`PartialEq`/`Eq` configuration comparison. The CLI flag
`--correlation_vector_limits` accepts a comma-separated partial override list of
`max_nodes`, `max_edges`, `max_events`, `max_metadata_values`,
`max_metadata_depth`, `max_metadata_bytes`, `max_input_bytes`, `max_work` and
`max_output_bytes`, each written as `name=unsigned_decimal`. Omitting the flag selects
all defaults (the mapped absent-flag string is empty). Explicit empty CLI values
retain cli-builder string validation; do not change shared parser coercion. Whitespace surrounding a pair/name/value is ignored; require ASCII
decimal digits, checked `usize` conversion and no duplicate or unknown names.
Reject empty pairs, missing values, signs, fractions and overflow. Enforce the
hard metadata-depth ceiling during configuration, even before tracing begins.
Zero is a meaningful limit for rejection tests. Explicit programmatic
configuration receives the same validation as CLI configuration.

When tracing is enabled, construct the checked compact log with these limits.
When disabled, retain disabled compact allocation behavior. Direct compiler-owned
CV operations propagate a `CompilerError::Provenance { stage: String,
message: String }` before any successful output publication. Diagnostics identify
the stage and preserve the underlying limit/identity/schema explanation; they
must not panic or discard the error. Existing typed frontend/pass failures retain
their compilation-stage diagnostics and underlying details, with failed
compilation status; do not infer categories from arbitrary message text.
Parser/configuration/execution failures route diagnostics to stderr with empty
success stdout; help/version remain successful stdout. No successful fallbacks
from formatter or summary errors are allowed. Graph validation remains required under format NONE
and before selecting filtered/summary views.

Whitespace-only minification retains its `Result<String, MinifyError>` API and
adds `MinifyError::Provenance(String)` for failed gap/emit tombstones. Both
pre-pass deletion and emit-loop skip recording use fallible deletion and stop
at the first failure. The compiler maps this variant to its provenance error
with the `whitespace_only` stage and failed-compilation status. Other minifier
errors retain their established diagnostics. Compilation-level and define
summaries propagate contribution failures; no ignored `let _` remains around
provenance mutations in any compiler stage.

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

## Borrowed checked export API

The library exposes `SourceFilter<'a> { sources: &'a [String], include_origin:
bool, invert: bool }` with an empty default, `SnapshotFormat::{CompactJson,
PrettyJson, Ndjson}` and `SummaryFormat::{Text, Json, Kv}`. The enums are ordinary
copyable selections; compact JSON/text are their defaults.
`CVLog::export_snapshot(&self, format: SnapshotFormat, filter: SourceFilter<'_>)
-> Result<String, String>` and `CVLog::export_summary(&self, format:
SummaryFormat, filter: SourceFilter<'_>, wrote_path: Option<&str>) ->
Result<String, String>` always validate complete graph evidence, including
ordinary controlled-constructor logs. Disabled, gapped or allocator-only imported
logs cannot enter these checked APIs. Preserve generic `to_json_string`
compatibility semantics separately.

Export validates the entire graph before applying a filter. An empty source list
selects everything, regardless of inversion/origin switches. Nonempty lists use
exact contribution-source membership, plus exact origin-source membership when
requested; inversion flips that decision. Every nonempty filter declares
`view:{complete:false,filtered:true}`, including filters which happen to select
all entries. Parents and all root allocator/stage metadata remain unchanged.
Projections are presentation views and cannot be reimported as complete graphs.

Validation, filtering, reference collection, sorting, counting and encoding share
one structural work allowance for each export. Charge before growing buffers,
copying references or sorting; source comparisons and examined contributions
also consume work. Borrow entries/metadata and allocate only charged reference
indexes, never a cloned graph or an intermediate JSON Value tree. Canonical
JSON sorts root, entry, record and nested metadata keys and preserves numbers
and arrays. NDJSON emits canonical entry records in identity order and one
canonical `_meta` footer containing every non-entry root field. Its newline
bytes count toward the same bounded sink. Pretty output includes indentation
bytes in that sink and never substitutes compact output on failure.

Summary exports count selected entries, contributions and tombstones but retain
the full graph's first-observed `pass_order`. Preserve the existing CLI text,
`{cv_sidecar:{...}}` JSON and `cv_sidecar.key=value` shapes, including path/skipped
semantics and trailing newline. Stream path quoting and comma-separated stages
through the bounded sink rather than materializing unbounded joined strings.
Count overflow and serialization/work/output caps are errors. The path describes
a successfully published sidecar only when the surrounding compiler publication
transaction commits; the library itself does not publish files. A zero output
cap rejects materialized snapshots and summaries, while NONE without summary
only validates graph evidence.

## Compiler filesystem publication transaction

The compiler prepares all requested bodies and validates/encodes provenance
before entering its private `publication` module. A transaction publishes JS,
source-map and manifest outputs first and its sidecar last. It provides complete
per-file installation and rollback on reported pre-commit failures; it does not
claim instantaneous multi-file visibility or crash/power-loss recovery.

Preflight every destination before changing existing outputs. Resolve existing
parent components through filesystem canonicalization and append only ordinary
missing components. Reject ambiguous missing-parent `..`, identical normalized
paths, Windows case aliases, existing-file identity aliases, and any destination
which is another destination's parent. Reject final symlinks, nonregular files
and read-only outputs. Parent symlinks may resolve to existing directories and
must participate in alias detection. Use filesystem object identity, not size,
timestamps or content equality: Unix device/inode; Windows volume plus full
128-bit FileIdInfo obtained from a live handle. Unsupported identity queries
fail closed. Retain handles during the transaction to prevent reuse of old IDs.
Use primitive FFI arguments and documented `repr(C)` buffers for Windows;
unsupported platforms fail explicitly rather than guessing identities.

Stage files in exclusively created sibling directories with fixed short names
`new`/`old`, a checked process-local sequence and bounded collision retries.
Unix private staging directories have mode 0700. Create stage files exclusively,
write complete bytes, sync, and preserve existing file permissions. Track each
owned path immediately; parent creation records only directories this operation
actually created. All staging completes before replacing any destination.

Back up the existing regular file only after rechecking its identity and observed
metadata. Create the backup with an exclusive no-clobber hard link, track that
path's creation immediately, then remove only the still-observed original
destination. An unexpected backup occupant must remain untouched, including
one which is a hard link to the original: identity alone does not establish
ownership of that pathname. Failure after creating the backup but before
removing the original recognizes the original as already restored and removes
only the backup link this operation created. Install the complete stage using a no-clobber
hard link: a newly appeared destination cannot be overwritten. Keep a private
stage link for ownership checks. Filesystems unable to provide the required
identity/hard-link semantics reject before claiming success. On a reported
pre-commit error, roll back in reverse order: remove only a destination whose
identity matches the held staged file, restore the original with a no-clobber
link and retain recovery copies when restoration is obstructed. Include the
original failure and every rollback/cleanup failure with exact recovery paths.
If no original backup was created, failed destination removal reports that
destination and its unconfirmed removal instead of naming a nonexistent recovery
file. Cleanup removes backup links only when their creation was recorded.
Never recursively delete a user directory or overwrite an unknown replacement.
Concurrency checks detect replacements at operation boundaries; this transaction
does not claim to serialize arbitrary outside modifications between syscalls.

The commit point is successful installation and identity/length verification of
all requested files. After that point, cleanup failures are reported as stderr
warnings naming retained owned paths, while the command remains successful:
the complete output set has already committed and deleting some backups cannot
be reversed. Before commit, cleanup errors accompany the failed command.
Ordinary success and rollback leave no owned temporary files. Empty created
parent directories are removed only on rollback and only if still owned.

`write_output_file` retains its standalone API; compiler artifact-set publication
uses the transaction. Existing output-I/O exit status is preserved. No successful
stdout or prepared summary is returned until publication commits. Candidate
`wrote` events become visible only with a committed sidecar and correspond to
complete content. Add `content_sha256` (lowercase 64 hex digits) alongside
`byte_len` to JS/map/manifest publication records, computed from the exact final
encoded bytes using the existing local SHA-256 package (promote its current
compiler dev-dependency; add no crates.io dependency). Source-root origins also
retain a digest of consumed UTF-8 bytes. Snapshot identity is not a self-hash;
do not create a circular sidecar digest. Hashing cost is proportional to source
and output bytes, independent of the structural graph-operation work allowance.

Process tests must first reproduce existing overwrite/collision defects, then
verify collisions including normalized/ancestor/hard-link/case aliases, failed
later staging with the full existing output set, exact published digests and
tracing-neutral bytes. Private unit hooks deterministically fail later installs,
rollback and post-commit cleanup, proving restoration, preserved unknown files,
retained recovery copies and truthful commit-versus-cleanup status. No production
failure-injection environment variable or CLI option is introduced. Independently
review the exact final code and execute required native-platform CI before
acceptance.

## Required verification

### Filesystem access-control refinement

Native Windows probes at published `90010c29de` found two access-control
failures despite successful ordinary tests: replacing a protected owner-only
output inherited BUILTIN Users read access, and both staging directory and
prepared file inherited Users read access before installation. Independent
probes reproduced both. Rust `Permissions` does not establish Windows DACL
preservation. The published PR remains draft until this refinement is implemented
and reviewed; its existing CI cannot accept these unresolved findings.

Create Windows staging directories and files atomically with a protected
current-user-only discretionary ACL. Keep each prepared file private throughout
preparation. Directory restrictions alone do not establish file privacy under
Windows traversal-bypass semantics. Preserve exclusive creation and held-object
identity checks; do not add a production failure-injection switch or dependency.

Capture the existing file's owner, group and discretionary ACL, including its
inheritance protection, from a held handle before staging. Recheck that policy
alongside identity/length/mtime at operation boundaries: an ACL change need not
alter those metadata fields. Preserve the supported policy exactly or reject it
before modifying originals. An initial implementation may reject non-current
owners or policies it cannot faithfully apply; it must not broaden access or
silently substitute readonly state. Define new destinations' intended policy
from their actual destination parent and creator, rather than inheriting the
private staging parent's policy accidentally.

Transition each candidate to its intended final policy during installation,
after its no-clobber destination link exists and before complete-set commit.
Use filesystem `SetSecurityInfo`, with the required access rights, then verify
the applied owner/group/DACL/protection on the held object. Account for parent
inheritance explicitly, including reopening and identity-verifying the installed
path when necessary. Setter/verification errors use the existing rollback path;
the original inode and its original policy remain recoverable. Granting intended
readers access after per-file installation fits the existing multi-file visibility
contract. Bound native descriptor, token, ACL and encoded-path buffers and retry
counts before growing caller-owned storage; reject unsupported states rather
than using an unverified fallback.

Native Windows round trips distinguish legacy explicit unprotected DACLs from
automatically inherited DACLs. `SetSecurityInfo` uses the opened handle's parent:
the private stage handle avoids adding grants to explicit legacy policies; an
identity-verified destination-parent handle supplies actual inheritance for
inherited/mixed policies. Treat this routing as a measured capability, not an
assumption that every inherited ACL is reproducible. Before modifying any
original, validate exact owner/group/ACE order/protection equality on a distinct
always-empty probe inode. Never apply a broad policy to a future output inode
and later tighten it before writing: an already-open reader retains access.
Reject stale/orphan inherited policies which cannot round trip. Restore each
candidate's private policy through its retained handle on pre-commit failure,
before pathname cleanup; report reset failures. Never reset candidate policy
during post-commit cleanup because stage and installed hard links share it.
The active empty probe must also permit the fresh metadata/security open used
by final verification, not merely readback through a retained privileged handle.
Reopen its active link with the production verification access mask, verify its
identity, zero length and exact intended policy before restoring probe privacy.
OWNER RIGHTS deny-READ_CONTROL policies must reject before any original mutation.

Unix mode bits alone also do not establish ownership or extended-ACL
preservation. State native Linux/macOS support separately, check owner/group and
extended/default ACL policy, and faithfully preserve it or reject unsupported
policy before modifying outputs. Do not treat all `cfg(unix)` targets as proven.
Full security-descriptor preservation, including audit SACLs, integrity labels
and claims, is outside an owner/group/DACL claim and remains unproven unless
separately specified and tested. Do not represent the narrower policy check as
complete filesystem-security preservation.

The initial Unix policy scope is native Linux and macOS, ordinary mode-based
files with no extended ACLs. Capture UID/GID/mode from held descriptors; reject
foreign owners and prove supported group assignment before changing originals.
Linux must query descriptor-bound POSIX access/default ACL attributes; macOS
must query descriptor-bound extended ACL entries. Reject unsupported extended
or parent-default policies before creating data-bearing candidates or changing
originals. Other Unix targets fail explicitly until separately specified.
Define a new file's intended owner/group/mode from a separate permanently empty
ordinary creation probe in its actual parent, preserving process umask and
filesystem group/setgid behavior without temporarily changing global umask.
Create data-bearing candidates with mode 0600 inside private 0700 directories.
Apply supported ownership before final mode at installation, verify exact
UID/GID/mode and continued ACL absence, and recheck original and parent policy
at boundaries. Pre-commit failure restores candidate privacy before cleanup;
original ownership/mode remain on the retained original inode. Native tests
must prove mode-only drift detection, existing ownership/mode preservation,
new-file creation policy and Linux/macOS extended/default ACL rejection before
original mutation. Cross-compilation is useful validation but is not native
ACL evidence; native CI is required before acceptance.

Native regressions must first reproduce protected-output access broadening and
prepared-file inherited grants. Verify atomically private stage files, existing
protected and unprotected/inherited policies, intended new-file policy, policy
changes which preserve id/length/mtime, setter/verification failures, rollback
and cleanup, and bounded/unsupported-policy rejection. Confirm actual ACLs and
readability without claiming a different-account test from a same-owner process.
Require native Linux/macOS copy-or-reject cases and another exact-head independent
security review before publishing the repaired head or accepting native CI.

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
