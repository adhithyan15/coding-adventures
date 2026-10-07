# CV03 — Actual bounded provenance chronology

## Scope and verified predecessor

CV02 merged in [#16905](https://github.com/adhithyan15/coding-adventures/pull/16905)
at `851ce71fc033f11d0c1716ed895b7026e5c64a81`, from reviewed head
`7debd2f26bd36ed243b9cf8273c6c576cd59a886`. Actual compiler commands passed on
Linux, macOS and Windows; both required gates and the full CI run completed
successfully, alongside CodeQL/test262/Books. Fetched-main reachability and exact
blobs/modes for all 120 reviewed paths were verified. The clean managed checkout
was archived and removed; the reviewed compiler binary and recoverable history
were preserved.

This selection starts at `f889e3db081c174cab17e890776dccd3ff274495`, whose
subsequent changes concern unrelated Mosaic/C preprocessing and LANG backlog
work. Preserve one active Closure/shared-stack PR. Introduce no crates.io
dependencies, and preserve the owner decisions: own AST, canonical base ESTree
boundary and the Babel converter. Commit this specification before tests and
implementation. This slice and its acceptance do not complete CCR-065 #15830,
#16868 or the compiler delivery contract in
[CLOSURE-COMPILER-RUST-BACKLOG.md](CLOSURE-COMPILER-RUST-BACKLOG.md).

## Verified problem and scope

At the unchanged compiler sources of reviewed/published 0e02820565,
`PassPipeline::run` executes repeated topological sweeps, including confirming
sweeps and OneShot passes. `PipelineOutput::execution_order` returns only the
distinct schedule. `CVLog::pass_order` records first contributor appearances,
and entry-local contribution lists have no global inter-entry clock.

The real ADVANCED input `function twice(x){return x*2;}
console.log(twice(7));` emits `console.log(14);` identically with tracing off/on.
Its scheduled constant-fold precedes inline, but the recorded `(7)*(2)` fold
must occur after inlining in a later sweep. The sidecar cannot identify that
sweep or establish global creation/contribution/deletion order. Its actual
source map remains empty, a separate required later implementation.

Deliver a complete ordered journal of accepted graph operations and actual
scheduler invocations, preserving existing graph validity, bounded ownership,
canonical exports, artifact rollback and compatibility. Do not reconstruct
execution history from CV identity allocation, graph topology, unique pass
names, metadata claims or final output. No graph-chronology claim establishes
complete source/composite/output-byte coverage or every mutating pass's lineage.

## Compatibility and coverage

1. Introduce a fresh explicitly opted-in checked compact constructor. The
   traced compiler uses it from the beginning; ordinary generic checked,
   compact/legacy and disabled constructors retain their established wire and
   recording contracts. Disabled tracing retains no journal/context strings.
2. A missing journal means chronology unavailable. Existing graph state and
   unstored allocated IDs cannot retrospectively become a complete journal.
   Reject attempts to enable recording on an already started log.
3. Add a versioned optional journal only to opted-in snapshots. Preserve
   absence versus present null/malformed/unsupported state. Generic snapshot
   loading must validate journal evidence through the checked path or reject
   unsupported presence; it must not silently discard this new field.
   Allocator-only imports cannot launder complete graph/journal evidence.
   `from_json_string` must accept valid full `chronology-v1` through the bounded
   checked importer with default limits, preserving all records and append
   state. Reject malformed, unsupported, disabled and partial declarations.
   Blanket rejection of every present journal is not delivery of this contract;
   add positive full round-trip and append controls through both import APIs.
4. Define coverage honestly: accepted graph operations are complete within an
   opted-in log; scheduler contexts describe actual recorded pipeline runs.
   Unscoped lexer/parser/compiler graph events remain globally ordered facts,
   not invented pass invocations. A missing composite identity remains missing.

## Typed records and actual contexts

5. Use an independent checked event sequence, encoded as fixed 16-digit
   lowercase hexadecimal strings across the full u64 domain. CV identity
   sequences remain independent. Complete accepted records are contiguous;
   rejected attempts consume neither allocator nor journal sequence/state.
6. Closed records distinguish root creation, derive, merge, contribution,
   deletion, context begin, accepted pipeline schedule and context end. Entity records reference existing
   graph facts and contribution indices without cloning arbitrary metadata or
   diagnostic values into another log. Preserve duplicate parent occurrences
   and actual operation kind. Each accepted contribution/deletion is referenced
   once; every stored node has one matching creation record.
   Preserve existing checked API semantics: create has zero parents, derive
   has one parent, and merge permits zero/one/many parents. Parent count alone
   cannot reconstruct the actual call kind. New children may derive or merge
   from known older tombstones; deletion forbids later contributions or a
   second deletion to that same entity, not references by later child nodes.
7. Context begin/end records identify real pipeline invocation, sweep index,
   scheduled pass position/name and FixedPoint/OneShot policy. The scheduler
   establishes context at invocation time. Include every unchanged and
   confirming invocation, repeated passes and repeated pipeline calls.
8. Scope pass context over `pass.run` AND acceptance of returned contributions
   and candidates. Record typed terminal success/failure and change status,
   distinguishing pass return from candidate acceptance. Restore enclosing
   context on error. Graph operations capture the active context internally;
   caller-edited metadata cannot assign an earlier invocation.
9. Pipeline outcomes distinguish convergence, fixed-point cap, scheduling
   failure and pass/recording failure. Do not fabricate an invocation when topo
   sorting failed. The existing execution_order/passes field remains distinct
   schedule inventory and its overclaiming documentation must be corrected.

## Bounds, atomicity and owned failure paths

10. Extend prospective accounting before any retained event/context allocation
    or mutation. Bound every record, retained string/reference, counter and
    event traversal using the existing finite event/payload/work/output/input
    caps. Node creation acquires a journal event charge in addition to node and
    parent charges. Specify contribution/deletion accounting exactly so their
    graph facts and journal references cannot escape structural caps.
    For opted-in logs define `usage.events = C + D + J + S`, where C/D are stored
    contribution/deletion facts and J is all stored typed journal records,
    including node/context events, and S is every retained pass descriptor in
    accepted schedule arrays. Ordinary checked logs retain C+D accounting.
    Outstanding terminal reservations R reduce available capacity using
    C+D+J+S+R; full import independently recomputes C+D+J+S. This intentionally
    charges both a graph fact and its separately retained journal reference,
    and prevents one schedule record from retaining an uncharged descriptor list.
    Typed fixed-size scalar storage is bounded by these record counts; arbitrary
    metadata retains its existing value/depth/byte limits. Owned journal names
    and any copied variable text also consume the payload byte allowance.
11. Context begin reserves terminal record slots, payload and sequence capacity.
    Intervening operations and nested contexts account against all outstanding
    reservations. Do not reserve a future fixed sequence that could collide;
    end records consume the next actual sequence. Failed begin leaves state
    unchanged; successful begin guarantees bounded terminal recording.
12. Terminal records use closed bounded outcome/change fields rather than
    cloned error strings. Preserve the primary pass error and restore parent
    context. A generic callback returning an arbitrarily deep owned result must
    not suffer recursive destruction because journal finalization failed.
13. Keep scheduler OwnedProgram protection, iterative rejected metadata
    disposal, candidate ownership and pending-contribution draining on all
    paths. Rejection may not accept a candidate or publish success-shaped
    artifacts after graph/journal failure. Apply the existing transactional
    output contract to chronology serialization failures.
14. A full success snapshot cannot be exported with active/unclosed contexts.
    Completed failure contexts remain explicit evidence that those invocations
    failed. An enclosing callback can catch a nested pipeline error and later
    return an accepted result; do not relabel the failed child or invalidate a
    complete closed journal solely because it includes such a child. Compiler
    success must come from the actual enclosing result and publication state,
    not from treating every closed context as successful. Panic-recovery semantics, if unsupported, must
    not advertise a recovered complete journal.

## Canonical validation, reload and presentations

15. Full checked import validates global sequence/state and one-to-one graph
    fact references, known earlier parent creations, operation-kind agreement,
    contribution index/source association, tombstone chronology, no later
    contribution/second deletion to a deleted entity, strict context nesting
    and context reference/order. Permit children referencing older tombstones.
    Reject duplicates, unknown refs, impossible outcomes and inconsistent
    allocation/state. Retain the existing duplicate-key, nesting, parse byte,
    conversion, graph, work and output limits as one bounded trust boundary.
16. Full JSON and pretty JSON canonically encode borrowed journal records,
    stable object keys and preserved chronological array order. No full Value
    clone or unbounded fallback serialization. Checked root-field parsing,
    canonical field counts and generic presence validation change together.
17. Existing NDJSON writes CVEntry lines and a final `_meta` footer. Preserve
    that for generic clients. Opted-in typed event lines must be unambiguously
    framed, with journal version/state in the final footer, and must not
    duplicate the entire event array there. Test lossless full reconstruction
    and checked reload with every sequence/reference preserved.
18. Filtered presentations validate original full evidence first and explicitly
    declare partial graph/journal coverage. Retain original event sequences and
    reference meanings; never renumber gaps into a fictitious complete history.
    Checked import must reject partial views as complete logs. Document retained
    context records and references to excluded graph facts precisely.
19. Summaries and NONE output share existing caps and failure propagation.
    Legacy snapshots remain readable with unavailable chronology; accessor and
    query results must expose that absence rather than infer a history.

## Meaningful acceptance evidence

Commit this finalized spec before tests/implementation. Add red tests covering
the current real multi-sweep gap, then implement rather than relabel it.

- Library interleavings across multiple entities: creation/derive/merge,
  contributions, deletion and nested contexts prove actual order independently
  from identity order, first-contributor order and DAG topology.
- Real scheduler multi-sweep fixed point with earlier constant-fold after inline,
  unchanged passes, OneShot repeats and final confirming sweeps. Actual observed
  invocation counters must agree one-for-one with the journal.
- Scheduling errors, pass errors, returned-contribution failures, context begin
  cap, reserved terminal capacity at/over limits, counter overflow and fixed-point
  cap: unchanged rejected state and explicit terminal outcomes, no false success.
- A deep returned AST and owned metadata on context/acceptance error paths are
  disposed safely on a small caller stack; test under real owned values rather
  than empty proxy results.
- Checked full JSON/pretty/NDJSON round trips and canonical repeatability;
  malformed sequence/ref/index/nesting/tombstone/outcome/coverage data; absent,
  null, unsupported and partial journal states; generic/legacy/disabled behavior.
- Actual SIMPLE/ADVANCED processes: journal evidence for real sweeps, identical
  traced/untraced JavaScript, fresh-process determinism, bounded error stderr,
  empty success stdout on failure and preservation of all old artifacts.
- Validate every directly affected shared Rust consumer, strict all-target lint,
  repository contracts/lessons, independent exact-head security review, actual
  native CI commands on Linux/macOS/Windows, and actual fetched-main merge.

Do not close the provenance epic or compiler umbrella after this slice. Exact
UTF-8/UTF-16/trivia/composite ownership, all fold families, inlining/motion/rename
and every mutating pass, real output/source-map joins and queries, modules/chunks/
externs/diagnostics/types/reports and five-level Closure parity remain required.

## Context and schedule validation details

A context reference is the sequence of its accepted begin record. It requires
no independent mutable invocation counter or caller-supplied context ID. Use
closed pipeline/pass context variants. Begin/end are controlled by a scoped
library operation that restores its parent on every ordinary success/error
path; callers cannot edit an active context or finalize an unrelated handle.
Pass contexts must be immediate children of their pipeline context. A nested
pipeline called from a pass has that pass as its parent and retains its own
schedule and sweep state. Ordinary graph events reference exactly the active
top context at their sequence, or null when no context is active. Import checks
this equality, rather than merely checking that a referenced context exists.

Begin the pipeline context before topological scheduling, with its terminal
capacity already reserved and its input Program protected by OwnedProgram.
An accepted schedule record follows successful topological sorting and precedes
any pass invocation. Store the bounded ordered pass name/policy descriptors
once, together with the positive configured sweep cap. Include the schedule
record in J accounting and charge retained variable names before copying them.
Duplicate names, impossible policies/indices and a second schedule for a run
reject. Scheduling failure has a pipeline begin/end pair but no accepted
schedule or fabricated pass invocation. Preserve the scheduler's existing
unknown-dependency behavior; chronology records the accepted schedule and does
not silently change dependency semantics.

For each completed sweep, all scheduled pass slots appear in increasing order
exactly once, even for OneShot or unchanged passes. Each pass name and policy
must agree with the accepted schedule. Sweep indices start at zero and are
contiguous. Another sweep is valid only if the previous completed sweep has a
successful FixedPoint pass reporting changed=true and the cap permits another.
Nested contexts may intervene without consuming the enclosing schedule's pass
slots. Convergence requires a complete final sweep with no FixedPoint change;
cap outcome requires exactly the configured cap of complete sweeps and a
FixedPoint change in the last one. An empty accepted schedule converges after
the scheduler's first empty sweep and has no pass contexts.

Failure permits only the actual prefix of a schedule. Distinguish a failed
pass callback, failure accepting its returned contribution/candidate, and a
recording rejection before a new pass context could begin. The last case must
not fabricate a pass begin. Closed terminal fields permit changed status only
where a returned candidate was actually accepted. A successful callback whose
candidate is rejected cannot become a successful pass record. Pipeline failure
closes its scope with the appropriate outcome; graph facts accepted before
that failure remain historical evidence and do not imply a successful run.
Import independently derives legal schedule progress and terminal outcomes
from records rather than trusting reported counters or diagnostic metadata.

A nested pipeline failure restores the enclosing pass context before returning
its error. The caller may catch that error, append further graph facts, invoke
another nested pipeline and return an accepted candidate. In that case the
failed child pipeline/pass remain failed, the later successful child remains
successful and the enclosing pass/pipeline may succeed. No error propagation
is inferred just from child failure. Propagated errors instead close each
enclosing scope according to its actual result. Full import accepts both
histories if scope, schedules, references and terminal outcomes are consistent.
Pass names need only be unique within one accepted schedule; the same name
across nested or repeated pipelines is valid and cannot identify an invocation.
Test active-context restoration by checking facts recorded after the caught
error and after the successful retry against the enclosing pass begin reference.

Reservable terminal completion must be infallible after successful begin for
ordinary success/error returns: all record, sequence, payload and vector
capacity needed by an end is purchased before invoking a callback. An end
references its begin and fixed typed outcome fields, without copying a pass
name or error string again. Intervening graph/schedule/nested-context mutations
must preserve every outstanding reservation. This protects generic returned
owned values as well as the scheduler's guarded AST; terminal completion must
not discard a deep successful result because an end unexpectedly ran out of
budget. Unsupported panic recovery leaves chronology explicitly unusable for
a complete export, rather than synthesizing a successful terminal record.

Validation establishes consistency and declared coverage of the stored
evidence. It cannot authenticate an arbitrarily rewritten whole artifact or
recover an operation that no instrumented caller recorded. The scheduler must
therefore supply these records at its real execution boundaries, and the
meaningful runtime/CLI tests must compare the journal against independently
observed invocation counters and actual resulting JavaScript.

## Concrete presentation framing

Initial API and wire names are fixed before the red tests:

- `CVLog::new_checked_chronology(limits)` creates a fresh checked compact log
  with complete chronology recording. `CVLog::journal()` returns an optional
  borrowed read-only journal; absence explicitly means unavailable. Its events
  and last sequence are exposed through read-only accessors. Callers cannot
  mutate records, switch recording or fabricate active context handles.
- The optional root `journal` object has exactly `version`, `coverage`,
  `last_sequence` and `events`. Version is `chronology-v1`; coverage is `full`
  or the explicitly unreloadable `partial` projection. Empty full journals have
  last sequence `0000000000000000` and an empty events array.
- Each record has exactly `sequence`, `context` and `event`. Sequence and
  non-null context references use the fixed hexadecimal encoding. A begin
  record's context references its enclosing scope; its own sequence becomes
  the new scope reference. Ordinary operations and schedule records reference
  the active top scope. An end references the scope it closes.
- `event.kind` is a closed tag: `create`, `derive`, `merge`, `contribution`,
  `deletion`, `context_begin`, `schedule` or `context_end`. Entity events carry
  `entity`; contribution additionally carries `index`. These reference graph
  facts and do not duplicate their parents, origin or metadata. A context begin
  carries a closed `scope`: pipeline, or pass with `sweep` and `slot`. Pass name
  and policy resolve from its enclosing accepted schedule. A schedule carries
  bounded ordered `passes` with `name`/`policy` descriptors and `sweep_cap`.
  Policies are `one-shot` and `fixed-point`.
- Context end carries its `begin` reference and closed typed `outcome`.
  Accepted pass outcomes record the actual accepted change bit; failed callback
  and failed acceptance outcomes do not claim an accepted candidate. Pipeline
  outcomes distinguish convergence, cap, scheduling, callback, acceptance and
  recording failure as specified above. Unknown fields/tags, wrong context
  kinds, noncanonical scalar encodings and cross-kind outcomes reject.

The scoped library operations may have private implementation helpers, but
they must restore enclosing state and purchase terminal capacity before
entering user callbacks. Schema or API refinements require an explicit
specification update committed before the corresponding implementation change.

Use one closed version marker for the optional full journal object, with its
fixed-width last-sequence state and chronologically ordered records. Empty
opted-in logs preserve present empty journal state; absent journal remains
unavailable. Reject present null, duplicate decoded fields, unknown versions,
unknown record variants/fields, numeric or noncanonical sequence values and
state that disagrees with the actual complete record coverage. A full imported
journal must be closed and must retain its checked limits and append watermark
so the next accepted event follows the last imported sequence exactly.

For opted-in NDJSON, keep ordinary CVEntry lines, then journal lines framed as
`{"_event": <typed-record>}`, then the existing final `_meta` footer. Put only
journal version/last-sequence state in the footer, not a duplicate events array.
Entry and event lines are distinguishable by their closed framing, not by
guessing from incidental entity fields. Full reconstruction preserves every
record and must pass the same bounded checked full importer; document whether
the library exposes a direct bounded NDJSON loader or the tested reconstructing
adapter, without implying a nonexistent import API.

Filtered journals retain all context begin/end and accepted schedule records,
and retain entity records only for selected entries. Existing selected entry
parent lists and original event/context references remain unchanged; excluded
parent graph facts are deliberately unavailable in the declared partial view.
Preserve sequence gaps and the original full journal watermark, mark graph and
journal coverage partial together, and reject full reload through both checked
and compatibility importers. Validate the complete original evidence before
selection and charge retained context/schedule traversal and event filtering
to the same export operation. Partial views cannot claim a complete sequence
merely because a filter happened to select every current entry.

Additional malformed-import tests must cover a valid-looking reference to a
closed or non-top context, pass/context nesting disagreement, mismatched name/
policy/index, missing/repeated/out-of-order slots, illicit extra sweeps, a false
convergence/cap result, acceptance failure labeled success, truncated schedule,
duplicate schedule and unknown record fields. Add successful nested-pipeline,
empty-schedule and append-after-full-reload controls, plus projected-view cases
with excluded parents and retained nested contexts. This closes consistency
holes that simple begin/end balance and existing-reference checks would miss.

Fresh scheduler baseline observation at reviewed/published c76de302f8, compiled
with Rust 1.99 against the tested package libraries, independently counted nine
actual callbacks across three sweeps (including unchanged FixedPoint and
OneShot passes). The scheduler returned only three distinct schedule names;
CV pass_order contained only one contributing source. A second call on the
same pipeline/log added three actual callbacks, for twelve total, while both
inventories remained unchanged. Complete graph validation passed, but the
snapshot still had no global journal. Preserve this independent observation as
the nested/repeated-run test's reference counters rather than deriving expected
callbacks from the future journal itself. This controlled scheduler witness
supplements the real ADVANCED inline/fold CLI witness; it does not establish
optimizer semantics, source-span coverage or a source-map implementation.
Baseline audit: `CV03-scheduler-observation-report.json` in the preserved CV02 audit directory. The committed regressions must independently reproduce these counts.

Use precision-safe encodings for contribution indices, pass slots, sweep
indices and configured sweep caps as well as event/context sequences. Fixed
16-digit lowercase hexadecimal values preserve the full supported u64 domain;
convert to usize only with a checked platform-width conversion before indexing.
Array positions remain implicit in the ordered schedule descriptors, so a
duplicated caller-supplied index cannot disagree with a second stored index.
Any repeated pass name/policy fields in a presentation must resolve from its
accepted schedule; do not retain an uncharged variable-text copy per sweep.

## Native boundary observations for red regressions

The source/head/library-hash-bound native witness
`CV03-nested-recovery-report.json`, executed with Rust 1.99 at reviewed published
`7debd2f26bd36ed243b9cf8273c6c576cd59a886`, verifies two real nested runs. Outer
and both inner passes share the name `same` and one graph root. Catching the
first inner callback error, retrying with a successful inner pipeline and then
accepting the outer callback produces three actual callbacks and seven accepted
contributions. Propagating the same inner error produces two callbacks and two
accepted historical contributions. Both graphs reload through the actual
bounded checked Rust importer and canonically re-export unchanged; both still
lack chronology and expose only the one-name contributor inventory.

Use the independently recorded callback timeline as the new journal oracle.
For this single-root, no-deletion witness the completed opted-in accounting is
`C7 + D0 + J23 + S3 = 33` in the recovered case: one root-create record, seven
contribution references, twelve begin/end records for three pipelines and three
passes, and three accepted schedule records. The propagated case is
`C2 + D0 + J13 + S2 = 17`: one root-create, two contribution references, eight
begin/end records for two pipelines and two passes, and two schedule records.
Outstanding terminal reservations must remain protected during the deepest
inner callback and after its error; end records consume those reservations in
the actual unwind order. The witness does not prove optimizer, raw-span or
output-map completeness.

Fresh Rust 1.99 execution against the tested libraries at published
`0dfbcf7b100ae187f7c9cadbc6ff06c986f1c914` independently observes eight boundary
cases. The observer records callback entry and return directly inside the pass,
then separately inspects the scheduler result and retained checked graph facts.
Every graph validates; every snapshot still lacks a journal.

| Case | Observed callbacks / successful returns | Actual scheduler result | Required journal distinction |
| --- | --- | --- | --- |
| Empty schedule | 0 / 0 | Success, empty inventory, no cap note | Accepted empty schedule, first empty sweep converged; no fabricated pass |
| Changed OneShot alone | 1 / 1 | Success, no cap note | Changed OneShot does not request another sweep |
| Always-changing FixedPoint plus OneShot observer | 200 / 200 | Success with cap note, two-name inventory, exactly 100 sweeps | Record every invocation and explicit cap outcome; preserve current note/result semantics |
| Callback failure | 1 / 0 | Original callback error, no contributions | Callback failure differs from candidate-acceptance failure |
| Returned-contribution cap failure | 1 / 1 | Scheduler error after one of two contributions is accepted | Successful callback return does not mean accepted candidate; retain first historical fact and close failed scope |
| Dependency cycle | 0 / 0 | Scheduling error | Pipeline begin/end with no accepted schedule or pass begin |
| Duplicate registered name | 0 / 0 | Scheduling error | Same zero-invocation prefix, typed scheduling failure |
| Missing dependency | 1 / 1 | Unknown dependency ignored, registered pass runs | Preserve existing dependency policy; do not manufacture a scheduling error |

The legacy returned-contribution witness uses `max_events=1`, which bounds its
one accepted contribution under ordinary C+D accounting. An opted-in journal
test must purchase the root, pipeline/schedule/pass records, schedule descriptor
and both terminal reservations first; reusing that legacy cap would reject
before invoking the callback and would test the wrong boundary. With no other
graph events, one root creation, one pipeline, one single-pass schedule, one
pass and one accepted contribution total C=1, D=0, J=7, S=1: nine retained event
charges after closure. During the callback the two reserved terminal slots
count against that same cap. A second returned contribution must fail atomically
without consuming its graph/journal charge or sequence, while both scoped ends
remain guaranteed. Check the exact accounting independently in the regression.

Preserved baseline artifacts: `CV03-scheduler-boundary-observation.rs`, its
read-only compile/run helper, and `CV03-scheduler-boundary-observation-report.json`.
The external report binds source/binary hashes and the actual tested library hashes; repository regressions must reproduce the observed behaviors, not depend on an external file.
These observations supplement the real ADVANCED inline/fold and nine/twelve
callback witnesses; they are not source-map, optimizer-parity or deep-drop proof.
