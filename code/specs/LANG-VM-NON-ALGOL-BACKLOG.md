# LANG VM non-ALGOL completion backlog

Status date: 2026-09-05

This is the execution backlog for completing the shared LANG VM platform while
the ALGOL campaign is owned separately. It complements
`LANG-FULL-IMPLEMENTATION.md`; when the two disagree about landed behavior,
executed tests and current package changelogs are authoritative until the older
roadmap is reconciled.

## VM-056 repair contract (discovered during VM-047b)

All five replacement programs fail on WASM (one bounds trap, four corrupted
outputs), while the other 30 cells and 292 oracle tests pass. INSPECT emits
`str_concat result = result, character`. The WASM runtime concat writes its
new allocation handle into the destination before reading source lengths and
bytes. When that destination aliases an operand, subsequent reads use the new,
uninitialized block rather than the original string.

Prioritize VM-056 before publishing VM-047b. Keep both operand locals intact
until the new header and bytes are written; defer destination assignment until
all reads and bump accounting finish. Preserve memory capacity checks and the
literal fast path. Add direct executable regressions for destination aliasing
the left, right and both operands. Re-run all 35 replacement cells, the full
WASM package suite, focused Clippy and the complete non-ALGOL matrix.

VM-056 local repair: the direct left/right/double-alias regression passes after
reproducing wrong output before the fix. All 35 replacement cells now pass in
fresh processes with positive sentinels and zero skips. All 236 WASM package
tests (including doctests), 292 INSPECT oracle tests and focused Clippy pass.
Full non-ALGOL matrix validation is running before publication.

Discovered follow-up **VM-057**: inspection found that runtime `str_slice`
also assigns its destination before reading the source during the byte copy.
An aliased source/destination may be overwritten similarly. Add a discriminating
runtime-parameter probe before changing this path; rank this suspected shared
lowering defect ahead of VM-047c when the current PR merges. It is not yet an
executed failure and is outside the concat repair's validated scope.

## VM-047b implementation contract (selected after #14400 merged)

Refreshed main is `2755b36eb5`. PR #14400 merged after all current-head checks
passed. No new runtime defect was discovered in tallying; replacement is the
next bounded proof in the existing priority queue.

Add five ASCII canonical replacement programs on the seven standard backends:
ALL using data-name operands and absent matches; LEADING stops at the first
mismatch while ALL reaches later matches; CHARACTERS replaces padded spaces;
a multi-item clause never feeds produced bytes into later items; overlapping
searches use the first written item. Observe repeated source writes and use
bracket markers where spaces matter. Add identical complete-source oracle
comparisons. Run each new cell in a fresh process with a positive sentinel,
all INSPECT oracle tests, focused Clippy, inventory counts and link checks.
Keep region boundaries in VM-047c. Any executed failure takes priority and
requires a committed repair contract before production changes.

## VM-047a implementation contract (selected after #14394 merged)

Refreshed main is `bf906aae9e`. PR #14394 completed VM-046c with all
current-head CI checks green; all three STRING/UNSTRING slices are complete.
No new executed failure outranks the existing coverage queue. Split VM-047:

| Order | Item | Bounded proof |
|---|---|---|
| done #14400 | VM-047a | ASCII INSPECT TALLYING ALL, CHARACTERS and LEADING on all seven standard backends. |
| selected | VM-047b | INSPECT replacement, including first-match and non-rechaining behavior. |
| then | VM-047c | INSPECT BEFORE/AFTER regions and absent-delimiter asymmetry. |

Add three canonical programs: ALL adds to a nonzero counter, a zero-match
inspection leaves it unchanged, and an item delimiter is observed; CHARACTERS
counts the full padded field width and accumulates; LEADING distinguishes an
initial run from later matches, handles no initial match and a fully matching
field. Use ASCII source and existing frontend/oracle semantics. Add identical
source/output oracle regressions for these combined observations. Execute each
new matrix cell in a fresh process with its single-cell sentinel, run the
INSPECT oracle suite and focused Clippy, and update the coverage inventory,
README and changelog. Any observed lowering defect gets a separate committed
repair contract before a fix. Replacement and region proofs remain later slices.

VM-047a local execution: rows 424–426 passed all 21 standard-backend cells
in fresh processes with positive single-cell sentinels and no skips. All 287
INSPECT JIT/oracle tests passed, including the three identical matrix sources.
Focused Clippy passed with warnings denied. No lowering repair was needed.
The corpus now contains 427 rows, including 47 COBOL rows; the non-ALGOL
capstone declares 195 programs and 1385 cells including 20 Twig BEAM cells.

## Prioritization policy

Re-rank before selecting every work item, and whenever current work discovers a
new gap. Apply this order:

1. Red executed conformance cells or silent backend skips.
2. Missing CI protection for already-working conformance.
3. Incorrect roadmap/status documentation that could send work down a dead path.
4. Missing backend parity for an already-implemented language feature.
5. New frontend semantics and historical-machine fidelity.

Each item must add or strengthen an executed cross-backend proof. One item uses
one fresh worktree and one PR; remove the worktree after merge.

## Completion loop and acceptance criteria

The loop keeps one active implementation PR. Before selecting another item,
fetch `origin/main`, inspect the previous PR's actual merge state, record new
discoveries, and rerank this backlog. Fix CI failures and merge conflicts on the
active PR. Publish ready-for-review PRs, never drafts, so draft-specific check
filters cannot suppress validation. Enable auto-merge only after its current-head checks finish green and
no conflicts or blocking reviews remain; do not bypass checks. After merge,
record completion and repeat from a clean feature worktree.

Completion means the scoped language features have executable conformance on
every applicable backend, those proofs run in normal CI, historical-machine
semantics are explicit and tested, and the status documents agree with the
code. A green example is not full language completion. The separate ALGOL
campaign retains ownership of ALGOL semantics. The platform vision's broader
tooling, transpilation, GC, and performance aspirations need their own audited
acceptance inventory (VM-029), rather than an implicit claim of completion.

### 2026-09-05 prioritization

The initial audit selected a directly observed CI gap: `lang-aot/BUILD` runs only the BASIC random differential
tests from `lang_matrix`, so the new non-ALGOL conformance rows are not protected.
The full matrix remains excluded for separately owned ALGOL Linux native-AOT
array failures. VM-024 will run every non-ALGOL `PROGRAMS` row and its declared
backends through the existing strict runner, preserving the full matrix and its
diagnostic single-cell interface. Missing tools remain explicit skips; compiler,
linker, runtime, and result failures remain failures. Report executed and skipped
cells and reject an empty selection. Validate on the local host and all PR CI
hosts before enabling auto-merge. Promote any discovered runtime failure ahead
of further coverage or semantic work.

Executing that selection exposed VM-030 on the very first Windows native cell.
PR #14265 repairs it; 62 library tests and all-target Clippy pass. Both LLVM
`lld-link` and Microsoft `link.exe` run the Windows smoke suite successfully
(eight reported tests, six exercised tests and two explicit GC early returns).
The original failing LANG cell `0:NativeAot` passes with the single-cell sentinel.
Current-head CI remains the merge gate. The subsequent matrix run reproduced
VM-033 (Windows text-output newlines), which now precedes coverage work. After
that repair, missing Windows runtime CI protection (VM-032) precedes the broader non-ALGOL matrix gate:
otherwise this exact regression can recur despite a green Windows job.

## VM-046c implementation contract (selected after #14387 merged)

PR #14387 merged as `8e52198a5b` after all checks passed at `3a0e0e870e`.
VM-046b and discovered VM-054/055 are complete. Full local non-ALGOL regression
passed 184 programs / 1308 cells with zero skips. Reprioritization selects the
remaining pointer/overflow slice; ALGOL stays separately owned.

Add canonical STRING and UNSTRING programs declaring all seven standard
backends. Observe receiver content, pointer writeback and distinct ON OVERFLOW /
NOT ON OVERFLOW markers together. Cover in-range offsets, exact fit, partial
transfer, invalid starting pointers, exhausted input and trailing-delimiter
boundary behavior using the existing frontend oracle contracts. Preserve
untouched receiver bytes with nonblank initializers and bracket output so
padding remains visible. Execute all new cells in fresh processes with ran-cell
sentinels; run frontend oracle tests and focused lint. Normal non-ALGOL BUILD
includes the rows automatically. Log and prioritize any new defect before
claiming parity, with repair specs committed before implementation. Update
inventory counts, README, changelog and validation evidence.

VM-046c local execution: all eight rows (416–423) pass all seven standard
backends in fresh processes with ran-cell sentinels: 56 executions, zero skips.
All 121 frontend STRING/UNSTRING oracle tests and focused matrix Clippy pass.
No runtime repair was needed. The inventory now declares 192 non-ALGOL
programs / 1364 cells; hosted normal BUILD remains the merge gate.

## VM-046b implementation contract (selected after #14377 merged)

PR #14377 merged as `b071b0493c` with all checks green at `3a68a24c33`.
VM-046a and discovered VM-053 are complete. Its full local non-ALGOL matrix
passed 179 programs / 1273 cells with zero skips. Reprioritization finds no
remaining observed red cell ahead of VM-046b; ALGOL stays separately owned.

Add bounded ASCII STRING delimiter and UNSTRING splitting programs on all
seven standard columns. STRING must stop each sender at its first delimiter,
including absent/leading delimiters and an item delimiter, while preserving
the receiver tail. UNSTRING must fit fields to receiver widths, space-fill
empty fields, drop excess fields and leave unused receivers unchanged after
source exhaustion. Use visible markers around receivers to retain padding and
empty-field evidence. Cover literal and item delimiters with existing oracle
semantics; pointer and overflow branches remain VM-046c. Execute each new cell
in a fresh process with a ran-cell sentinel, run frontend oracle checks and
focused lint, and update source-linked inventory counts. Normal non-ALGOL BUILD
must include every row. Log and prioritize any actual backend defect before
claiming these cells proven; commit repair contracts before implementation.

### VM-054/055 discovery and repair contracts (block VM-046b)

The 35-cell survey passes all 25 WASM/JVM/CLR/VM/JIT cells, but every native
and LLVM delimiter cell fails. LLVM refuses nonconstant `str_index` loop
indices (VM-055). Native produces empty output (VM-054): its string-lowering
integer facts fold a loop-carried scan index to its initial constant instead
of reading the current value. The frontend oracle selection passes 121 tests.

Prioritize both failures within this slice. Native string folding must exclude
multiply defined integer registers and invalidate overwritten integer facts;
nonfoldable str_index operations must use the existing bounds-checked runtime
helper, registered for both native ABIs. LLVM must route runtime sources or
indices to that same helper, retaining literal fast paths and correct scalar
result facts. Link the production helper in the matrix. Add focused regressions
for loop-carried indices and runtime sources, execute the new cells, and rerun
the full non-ALGOL matrix. Do not remove backend declarations or change COBOL
semantics to hide failures.

VM-046b/054/055 local validation: all five new rows (411–415) execute on
all seven standard backends with fresh-process ran-cell sentinels: 35 cells,
zero skips. Frontend STRING/UNSTRING oracle selection passes 121 tests;
220 native library tests and 128 LLVM integration tests pass. Focused matrix
and all-target affected-backend Clippy pass. Full non-ALGOL regression is
running; hosted CI remains the merge gate. The inventory declares 184
non-ALGOL programs / 1308 cells.

## VM-046a implementation contract (selected after #14370 merged)

PR #14370 merged as `8504430394` after all current-head checks passed on
`b878adec71`. VM-045 and the discovered VM-050/051/052 repairs are complete.
The full local non-ALGOL matrix passed 176 programs / 1252 cells, zero skips.
No red cell supersedes the next coverage item. Split VM-046 into small proofs:

| Priority | Slice | Acceptance |
|---|---|---|
| done #14377 | VM-046a | STRING DELIMITED BY SIZE: full source widths/spaces, mixed literal/item input, receiver truncation and preservation of a nonblank untouched tail. |
| done #14387 | VM-046b | STRING delimiters and UNSTRING basic splitting: empty fields, receiver fitting and untouched receivers after source exhaustion. |
| done #14394 | VM-046c | STRING/UNSTRING pointer updates and overflow branches with distinguishable outputs. |

VM-046a adds canonical ASCII programs declaring all seven standard backends.
Use visible trailing markers so output normalization cannot hide spaces, and
nonblank initial receiver tails to distinguish STRING from MOVE space-filling.
Run each new cell in a fresh process with a ran-cell sentinel; validate borrowed
semantics against the frontend oracle cases. Normal non-ALGOL BUILD includes
new rows automatically. Preserve frontend/runtime semantics; record and repair
any newly observed defect before claiming parity. Update inventory counts,
README, changelog and backlog evidence. ALGOL remains separately owned.

### VM-053 discovery and repair contract (blocks VM-046a)

New row 410 executes repeated STRING with a changed source. Native/LLVM print
`ABZZZZ|` then `CDZZZZ|`; WASM incorrectly prints the later value twice.
Rows 408/409 pass all seven backends, and 121 frontend STRING/UNSTRING oracle
tests pass. Reprioritize this observed stale string-fact defect before coverage.

WASM's function-wide literal table must not substitute a later assignment for
an earlier read, even when both assignments occur in one basic block. Mark
multiply written string variables as runtime-valued and propagate that status
to their consumers; preserve the single-definition literal fast path. Add a
focused regression proving reassigned literals are runtime values, rerun all
new cells and the full non-ALGOL matrix, and report actual execution/skips.
Keep COBOL semantics and the other backends unchanged.

VM-046a/053 local validation: rows 408–410 each pass all seven standard
backends in fresh processes with ran-cell sentinels (21 executions, zero skips).
The frontend STRING/UNSTRING oracle selection passes 121 tests. The complete
WASM package suite passes 235 tests including doctests. Focused matrix and
all-target WASM Clippy pass. The broader non-ALGOL matrix is running; hosted
CI remains the merge gate. Inventory now declares 179 non-ALGOL rows / 1273 cells.

## VM-045 implementation contract (selected after #14363 merged)

PR #14363 merged as `7291d2684e` after all current-head checks passed on
`afe88dca63`. VM-044 is complete. Reprioritization selects the next existing
coverage gap, VM-045; ALGOL remains separately owned.

Add canonical COBOL ASCII reference-modification programs for literal and
computed start/length, omitted length, IF/EVALUATE comparisons and MOVE into
wider/narrower alphanumeric receivers. Include trailing markers to preserve
padding evidence. Add runtime-invalid start and end cases expecting traps,
matching the existing JIT/oracle bounds contract. Declare all seven standard
backends, execute each new cell in a fresh process with a ran-cell sentinel,
and validate the borrowed semantics against the frontend oracle suite. Normal
non-ALGOL BUILD must include these rows automatically. Do not broaden existing
byte/character or receiver-category semantics. If execution exposes a backend
defect, record and prioritize a bounded repair before claiming parity. Update
inventory counts, README, changelog and validation evidence.

### VM-050 discovery and repair contract (blocks VM-045)

Executing VM-045 reproduced `BackendRefused { function: "main" }` on native
AOT row 402 with computed substring indices. Literal row 401 passed all seven
backends; the frontend reference-modification oracle suite passed 66 tests.
Native string lowering only folds constant slices and leaves runtime slices
for a backend that refuses them, despite the existing bounds-checked
`__twig_str_slice` runtime helper. Prioritize this observed failure within the
current VM-045 PR before declaring its new cells proven.

Route well-formed slices that cannot be folded through `call_builtin str_slice`
and invalidate stale destination string facts. Preserve constant folding and
its invalid-bound traps. Verify runtime output and invalid-bound failures via
the canonical rows, plus focused lowering checks for runtime source/bounds and
reassigned destinations. Use existing runtime/ABI helper plumbing; investigate
any further backend refusal instead of excluding its cells.

### VM-051/052 discovery and repair contracts (block VM-045)

The complete 49-cell survey after VM-050 passed 46 cells. LLVM rows 402 and
405 reject computed `str_slice` bounds as nonconstant (VM-051). WASM row 405
prints `|` and two spaces plus `|` instead of `BC   |` / `BC|` (VM-052).
All seven native cells pass, including both expected traps. Prioritize these
observed failures within this PR before declaring the new rows protected.

VM-051: use LLVM's existing length-prefixed runtime string representation and
shared bounds-checked slice helper for unknown sources/bounds. Preserve its
literal folding, trap behavior and stale-fact invalidation. VM-052: diagnose
runtime MOVE's incorrect WASM output and fix the narrow string-lowering fact
or copy defect responsible, with the failing canonical program as regression
proof. Do not alter COBOL semantics or remove failing backend declarations.

VM-045/050/051/052 local validation: all seven new rows (401–407) pass all
seven standard backends in fresh processes with ran-cell sentinels: 49
executions, zero skips. The frontend oracle suite passes 66 tests; affected
backend libraries pass 281 tests, and LLVM integration passes 127 tests.
Native and LLVM now route computed slices through the production checked
runtime helper. WASM propagates runtime representation from computed bounds
through downstream string copies. Hosted CI remains the merge gate.

## VM-044 implementation contract (selected after #14358 merged)

PR #14358 merged as `669d5fcc1e` after all current-head checks passed on
`a9f733229e`. VM-037 is complete; no new red cell supersedes the existing
coverage queue. Select VM-044, leaving ALGOL with its separate owner.

Add canonical Oct programs that expose while-loop iterations and a returned
function value through stdout, including u8 wrapping across iterations. Add
loop/conditional-break and nested-break programs whose output distinguishes
inner from outer targets. Declare all seven standard backends and run every
new cell in a fresh process with an execution sentinel, reporting absent tools
honestly. Normal non-ALGOL BUILD must automatically cover the new rows.
Preserve frontend/runtime semantics; if a new program exposes a real defect,
log it and prioritize a bounded fix before promoting the affected cell.
Update inventory counts, changelog and backlog evidence. Byte wrapping and
calls must be observed at runtime, not merely validator acceptance.

VM-044 local execution: rows 373, 374 and 375 passed on all seven standard
backends, each in a fresh process with the ran-cell sentinel (21 executions,
zero skips). No runtime fix was needed. The corpus now declares 12 Oct rows
and 169 non-ALGOL programs / 1203 cells. Hosted CI remains the merge gate.

## VM-037 implementation contract (selected after #14349 merged)

PR #14349 merged as `839e4191e8` after all applicable checks passed on
`983ab9fbfe`. Hosted Windows explicitly reported 19 McCarthy native programs
executed; Linux/macOS BUILD also passed. VM-036 is complete. Reprioritization
selects VM-037, the next coverage-only item; ALGOL remains separately owned.

Add canonical FLOW-MATIC programs proving a taken EQUAL branch, untaken
LESS/GREATER branches reaching OTHERWISE, and an unconditional jump chain.
All paths must terminate and wrong paths must print distinguishable output;
use differing record widths/line counts rather than infinite-loop failure
sentinels. Fields start at zero, so this slice does not claim positive LESS or
GREATER comparisons over nonzero input (the EOF-aware input work is VM-039).
Declare all seven standard backends and execute each new cell with strict
existing result assertions, reporting missing tools honestly. Normal non-ALGOL
BUILD must include the new rows automatically. Preserve frontend/runtime
semantics. Update source-linked inventory counts and backlog validation.

VM-037 local execution: new canonical rows 374, 375 and 376 each passed
NativeAOT, LLVM, WASM, JVM, CLR, VM and JIT in fresh single-cell processes.
All 21 executions produced the required ran-cell sentinel; zero tool skips.
The corpus now has four FLOW-MATIC rows and 166 non-ALGOL programs declaring
1182 cells. Hosted CI remains the merge gate.

## VM-036 implementation contract (selected after #14341 merged)

The ten-frontend audit merged as `1fcd588c6b` after all checks passed on
`23dc4ec216`. No new runtime failure supersedes the coverage queue. Select
VM-036: McCarthy's native capstone should execute on Windows and Linux, not
return None solely because the host is not macOS. ALGOL remains separately owned.

Use each host's existing executable compilation API. Probe the matching linker:
Windows must distinguish Microsoft/LLVM/MinGW from POSIX link.exe; Linux honors
CC as the production linker does; macOS probes ld. Once the linker is detected,
compile/link/launch or result errors must fail loudly. Preserve the existing
19-program corpus and exact result comparisons (including process failures).

Add a native-only test over all 19 programs for the existing Windows native CI
step. It must assert nonempty/full execution, emit a count, and fail if
LANG_REQUIRE_WINDOWS_AOT=1 but no Windows linker is found. Normal Linux/macOS
BUILD already runs the conformance target. Validate the focused corpus locally
with the real Microsoft linker and LLVM linker, and prove the required-linker
negative path fails with an empty PATH in a child invocation. Update coverage
docs and retain all existing standard/managed conformance lanes.

VM-036 local validation: all 19 native programs executed with Microsoft
link.exe, then all 19 with an LLVM-only linker PATH. With an empty PATH and
LANG_REQUIRE_WINDOWS_AOT=1, the same test failed at the required-linker assertion
(exit 101, zero executed programs). All-target lang-aot Clippy with warnings
denied and six Windows CI selector tests passed. Linux/macOS execution and the
new hosted Windows command remain required PR CI evidence before merge.

## VM-027 implementation contract (selected after #14333 merged)

PR #14333 merged as `cd73f3ad86` after Linux, macOS, Windows and the CI gates
passed on `422ad4c847`. No new runtime defect was exposed by VM-035. With
VM-025 still separately owned, reprioritization selects the ten-frontend
feature/backend inventory before new Oct or Nib machine semantics.

Produce a source-linked inventory for every wired Language variant. Distinguish
frontend implementation, declared executable corpus coverage, clean refusals,
missing tests and absent-tool skips; a wired backend is not full feature parity.
Include dedicated McCarthy and Macsyma suites, platform restrictions and exact
normal BUILD commands. Compare frontend lowerers/oracle tests to the matrix
rather than inferring completeness from README headings or row counts. Preserve
ALGOL ownership and record its full-matrix gate separately.

Correct stale driver and conformance scope comments. Log each uncovered feature
family as a bounded follow-up with an executable acceptance criterion, grouping
only features sharing a lowering/runtime boundary. Prioritize observed failures
and missing CI protection before expanding semantic scope. Validate inventory
links and corpus counts against source; execute representative dedicated-suite
proofs for the coverage outside the unified corpus, reporting actual skips.

## VM-035 implementation contract (selected after #14325 merged)

Local validation: the new isolated-target regression failed against the old
helper with a nonexistent default archive, then passed after Cargo artifact
discovery was implemented (three parent/parser tests; the ignored child is
explicitly executed by its parent). The original Twig heap cell `2:Llvm`
compiled and ran with its single-cell sentinel in both a fresh target directory
containing spaces and the default directory. All-target lang-aot Clippy passed
with warnings denied. Current-head hosted CI remains the merge gate.

PR #14325 merged as `bf9043d695` after all applicable checks passed on
`618579d847`. Reprioritization selects the reproduced archive lookup defect
VM-035 before the larger VM-027 feature audit; VM-025 remains separately owned.

The shared `lang-aot/tests/common` GC archive helper must obtain the staticlib
path from the successful nested Cargo build's JSON compiler-artifact messages.
Match the `gc_core_capi` staticlib target and its `.a`/MSVC `.lib` archive,
excluding rlibs, dynamic libraries and import libraries. Reject malformed,
missing or ambiguous archive messages and nonexistent output paths; never fall
back to a guessed workspace/target filename. Preserve actionable compiler
failure diagnostics and honor Cargo's target-directory/configuration choices.

Use the existing Cargo build as the authority, with a test-only JSON dependency.
Add a normal BUILD regression that launches a child test process with a fresh
`CARGO_TARGET_DIR` containing spaces. It must build and find the actual archive
under that directory, without mutating the parent test process environment.
Verify JSON selection for Unix and MSVC names and unrelated/no/ambiguous
artifacts. Confirm the original heap LLVM matrix cell runs in a fresh process
with a nondefault target directory. Preserve default-layout use, other test
runners, production runtime semantics and separately owned ALGOL behavior.

## VM-026 implementation contract (selected after #14317 merged)

PR #14317 merged as `6fcc258b32` after every applicable current-head check
passed on `763e68acaf`. VM-024 is complete. VM-025 remains with the separate
ALGOL campaign, so select the highest-priority unowned item, VM-026. The
alternate-target helper issue VM-035 remains recorded for its own bounded fix.

Reconcile `LANG-FULL-IMPLEMENTATION.md`, `LANG-PLATFORM-MATRIX.md`, and the
umbrella `LANG-PLATFORM-VISION.md` with current source and executed evidence.
Correct stale Twig symbol/quote, forward-global, and boxed-global arithmetic
gaps and the obsolete VM-014 work pointer. Distinguish ten wired Language
variants from the eight languages represented in the unified corpus and the
dedicated McCarthy/Macsyma suites. Describe seven standard engines separately
from additional declared BEAM cells. Retain historical LM0 milestones as
history, not claims of full current language completion. Fix the directly linked
E6 dispatch status where it repeats the same already-landed gap.

Link current runner/test/BUILD boundaries and the merged runtime/coverage PRs.
Keep genuinely unverified feature coverage under VM-027, platform vision work
under VM-029, and separately owned ALGOL semantics unchanged. Validate source
counts, named regression rows, local document links and diff cleanliness;
reuse #14317's executed 163-program/1,161-cell evidence rather than inventing
new completeness claims or redundant runtime tests for a documentation edit.

## VM-024 implementation contract (selected after #14295 merged)

PR #14295 merged as `b4535bf5ab` after final head `17a5faa492` passed all
applicable checks, including actual execution of the dedicated Windows native
step. With VM-030, VM-033, VM-034, and VM-032 landed, reprioritization returns to
VM-024 as the highest remaining unowned coverage item.

Add a normal BUILD invocation selecting all non-ALGOL rows from the canonical
`PROGRAMS` corpus and every backend each row declares. Preserve the complete
matrix and diagnostic single-cell protocol unchanged. Use the existing strict
runner and result assertions, including the portable text comparison. Count
programs, executed cells and missing-tool skips; reject empty selections and
any runner returning no result after its toolchain was detected. Keep this
fail-fast so one in-process failure cannot contaminate subsequent results.

Run the entire selection on the local host and let normal PR builds validate
Linux/macOS. Windows's dedicated native proof remains VM-032; this item does
not claim the whole language matrix runs in the Windows Rust-only CI leg.
If execution exposes another runtime failure, record its exact program/backend,
confirm it in a fresh single-cell process, and promote a bounded repair before
publishing the wider coverage gate. Full ALGOL CI remains separately VM-025.

Local Windows validation passes the complete selected corpus: 163 programs,
1,161 executed backend cells, zero skips, in 351.52 seconds. All-target
`lang-aot` Clippy and security review pass. The first alternate-target-layout
attempt is recorded as VM-D025/VM-035; the passing result uses Cargo's normal
workspace target directory. Hosted current-head CI remains the merge gate.

## VM-032 implementation contract (selected after #14289 merged)

PR #14289 merged as `e6765d7711` after all applicable checks passed on
`92250cbe12`. VM-033 and VM-034 are complete. Reprioritization now selects
VM-032 ahead of VM-024: Windows must execute its native regression on affected
PRs before widening the general matrix coverage.

Derive a dedicated LANG Windows runtime flag from the build plan's Windows
`affected_packages` override, falling back to the top-level affected closure
only when there is no Windows override. Select `rust/twig-aot` or
`rust/lang-aot`; the planner already propagates changes through their dependency
closure. A null or missing closure means full coverage; an empty closure means
no affected packages. Reject malformed plans. Changes to the workflow, selector,
its tests, or the MSVC bootstrap must self-select the gate even for an empty
package plan.

Wire the flag into Windows runner selection, Rust setup, and the existing MSVC
developer-environment setup. Add a dedicated Windows step running
`cargo test -p twig-aot --test windows_x86_64_smoke -- --nocapture` with
`LANG_REQUIRE_WINDOWS_AOT=1`. In that mode absence of a real linker must fail,
not silently skip. Preserve optional local-toolchain skips without the flag and
the two explicit, separately tracked precise-GC early returns (VM-031).

Validate selection against Windows overrides, unrelated and full plans, and
self-changes. Exercise the real Windows suite with the required flag, then run
its scalar test binary with no linker on PATH and prove a nonzero result.
Validate workflow wiring and the existing planner/metadata contracts. Actual
hosted Windows runtime execution on the PR is the final acceptance evidence.

Local validation: all eight smoke tests report success with the required flag;
five launch programs, one checks the PE object, and two explicitly return for
VM-031. Removing every linker from PATH makes the scalar test fail with exit
101 in required mode, while optional local mode retains its documented skip.
Six selector/wiring tests, 15 CI-registry tests, and five MSVC-bootstrap tests
pass. The recorded #14265 plan selects this gate, Ruff and YAML parsing pass,
and all-target `twig-aot` Clippy is clean. Hosted CI remains the merge gate.

## Ranked backlog

### VM-033 implementation contract (selected after #14265 merged)

PR #14265 merged as `0e18482307` after every applicable current-head CI check
passed. Reprioritization selects the independently reproduced `352:Llvm`
stdout mismatch before VM-032 and VM-024 coverage work.

`Expect::Stdout` describes text lines for the text-output languages: compare
CRLF as LF, on either host, without removing lone carriage returns, spaces,
tabs, empty lines, or Unicode characters. Brainfuck is the byte-oriented
exception and retains the existing exact comparison. Keep the raw observed
stdout in failure diagnostics. Apply the same comparison boundary to the BASIC
full-value differential tests, so the matrix and differential harness agree.
This changes test interpretation only; do not change compiler or runtime output.

Validate positive LF/CRLF equivalence and negative cases for altered content,
lone CR, and Brainfuck output. Execute the existing BASIC multi-line real,
mixed DATA, and RND rows on every available declared backend, requiring every
available runner to return a result. Add those focused tests to normal BUILD;
the complete non-ALGOL corpus remains VM-024 and full ALGOL coverage VM-025.

The new RND regression exposed VM-034 before publication. The focused repair
must include this prerequisite rather than excluding that backend or program:
the JVM's simulator-compatibility pass must preserve integer widths in modules
that use floating-point values. Such modules cannot run on the legacy 32-bit
integer-only simulator anyway. Keep this decision module-wide so helper calls
and globals agree; prove that RND's `i64` product remains wide and execute the
existing seed/repeat/advance transcript on real Java. This is a generic mixed
numeric-type boundary, not a BASIC-specific RNG special case. Preserve the
existing integer-only simulator path and run its regression suite.

Local validation on Windows: the LF/CRLF regression fails before the fix and
passes after it; three focused tests pass with 18 executed backend cells and
three missing-tool skips. The library and five JVM suites pass 24 tests,
including the integer-only simulator and real-Java Lisp tests. BASIC T7 tests
exercise 400 generated programs with 864 cross-engine agreements. The ALGOL
real-procedure and real-for-variable matrix regressions pass on available
backends, and all-target Clippy is clean. Full-matrix completion is not claimed.

| Rank | ID | Status | Work item | Completion proof |
|---:|---|---|---|---|
| — | VM-001 | done ([#13306](https://github.com/adhithyan15/coding-adventures/pull/13306)) | Preserve COBOL scale-12 decimal intermediates on JVM. | The existing `A / B + C` LANG matrix cell prints `000533` on real Java and the full matrix is green. |
| — | VM-004 | done ([#13340](https://github.com/adhithyan15/coding-adventures/pull/13340)) | Normalize signed process results in `macsyma_conformance` for LLVM and native AOT. | `-7$` compares as `-7`, not process status 249, and the full Macsyma conformance suite passes on every available backend. |
| — | VM-005 | done ([#13348](https://github.com/adhithyan15/coding-adventures/pull/13348)) | Link Linux LLVM matrix executables with the host math library. | Dartmouth BASIC `trunc` and non-ALGOL math cells link and pass under `lang_matrix` on Linux; ALGOL-native gaps remain separately owned. |
| — | VM-002 | done ([#13326](https://github.com/adhithyan15/coding-adventures/pull/13326)) | Protect every CI-green `lang-aot` integration suite in `lang-aot/BUILD`, including `e6d7a_wasm_closures`; replace stale exclusions with the current Macsyma and Linux-matrix failures. | The normal build command runs every CI-green top-level integration target and documents both red exclusions precisely. |
| — | VM-003 | done ([#13356](https://github.com/adhithyan15/coding-adventures/pull/13356)) | Reconcile `LANG-FULL-IMPLEMENTATION.md` with landed Twig E6 and McCarthy work. | Roadmap statuses match current executed suites and name only genuine gaps. |
| — | VM-015 | done ([#13367](https://github.com/adhithyan15/coding-adventures/pull/13367)) | Reconcile the McCarthy compiler README with the completed L3/W16 backend campaign. | The package README no longer calls L3 future work and links the authoritative eight-backend completion matrix. |
| — | VM-010 | done ([#13380](https://github.com/adhithyan15/coding-adventures/pull/13380)) | Close the remaining Twig dynamic VM/JIT gaps: interned symbols/quote and forward-referenced globals, including boxed global arithmetic. | The existing symbol and dynamic-global matrix cells run on VM and JIT in addition to the five code-generation backends. |
| — | VM-011 | decomposed | Close Dartmouth BASIC's non-ALGOL semantic tail: remaining math builtins and dynamic string data paths. | Split into VM-016 through VM-018 because deterministic transcendental wiring, mixed-type `DATA`, and portable randomness have independent designs and risk. |
| — | VM-016 | done ([#13392](https://github.com/adhithyan15/coding-adventures/pull/13392)) | Wire Dartmouth BASIC's deterministic `SIN`, `COS`, `LOG`, and `EXP` builtins to the existing shared transcendental IIR ops. | One discriminating program executes all four functions on NativeAOT, LLVM, WASM, JVM, CLR, VM, and JIT. |
| — | VM-019 | done ([#13419](https://github.com/adhithyan15/coding-adventures/pull/13419)) | Reconcile the Dartmouth BASIC compiler README with landed general exponentiation, seven-backend string arrays, and current dynamic-string limitations. | The README no longer directs work toward already-landed power or string-array support and names only executed current gaps. |
| — | VM-014 | decomposed | Extend the unified matrix to every wired frontend, especially FLOW-MATIC and Macsyma/JIT. | Split into VM-020 and VM-021 because FLOW-MATIC needs an executed matrix baseline while Macsyma needs new universal-JIT runtime glue. |
| — | VM-020 | done ([#13428](https://github.com/adhithyan15/coding-adventures/pull/13428)) | Add FLOW-MATIC's first unified matrix baseline. | A field `MOVE` plus `WRITE-ITEM` program prints `0` on NativeAOT, LLVM, WASM, JVM, CLR, VM, and JIT. |
| — | VM-021 | done ([#13442](https://github.com/adhithyan15/coding-adventures/pull/13442)) | Add Macsyma to the universal JIT and its cross-backend conformance suite. | Macsyma's integer arithmetic corpus agrees across VM, NativeAOT, LLVM, WASM, JVM, CLR, and JIT. |
| — | VM-017 | done ([#13452](https://github.com/adhithyan15/coding-adventures/pull/13452)) | Add mixed numeric/string Dartmouth BASIC `DATA`, `READ`, and `RESTORE` semantics. | Scalar and array string reads preserve source order with numeric values and execute on all applicable standard backends. |
| — | VM-022 | done ([#13762](https://github.com/adhithyan15/coding-adventures/pull/13762)) | Repair the TypeScript Dartmouth BASIC parser `BUILD` so it runs that package's tests instead of ending in the generic parser package. | `BUILD` executes the Dartmouth parser suite itself and includes a mixed numeric/string `DATA` regression. |
| — | VM-023 | done ([#13773](https://github.com/adhithyan15/coding-adventures/pull/13773)) | Audit and repair the same stateful-directory build defect across non-ALGOL TypeScript parser and lexer frontends. | Every affected package's normal and Windows build scripts run that package's own tests, with an automated guard against ending in a dependency directory. |
| — | VM-012 | done ([#13785](https://github.com/adhithyan15/coding-adventures/pull/13785)) | Implement Nib BCD storage semantics and Intel-4004 RAM mapping. | Hardware-faithful programs agree across the portable matrix and the 4004 simulator. |
| — | VM-018 | done ([#13802](https://github.com/adhithyan15/coding-adventures/pull/13802)) | Define and implement portable Dartmouth BASIC `RND` semantics. | Merged as `8ceb8efc60`; the matrix includes negative reseeding, positive advancement, zero repeat, and shared `DEF FN` state on seven backends. |
| — | VM-030 | done ([#14265](https://github.com/adhithyan15/coding-adventures/pull/14265)); discovered by VM-024 | Repair Windows native-AOT CRT linking. | Merged `0e18482307`; both Windows linkers execute the smoke suite, 62 unit tests and Clippy pass, and Linux/macOS/Windows CI and both CI gates finished green. |
| — | VM-033 | done ([#14289](https://github.com/adhithyan15/coding-adventures/pull/14289)) | Define portable text-output comparison in the LANG matrix. | Windows LLVM BASIC cell `352:Llvm` prints correct values with CRLF but the LF expectation fails; normalize only the accepted host text-newline difference, preserve meaningful output bytes, and run discriminating multi-line cases on available backends. |
| — | VM-034 | done ([#14289](https://github.com/adhithyan15/coding-adventures/pull/14289)) | Preserve JVM integer arithmetic in mixed floating-point modules. | RND cell `360:Jvm` must produce `22`, `85032`, `85032`, `601352`; its `48271 * 48271` intermediate stays i64, while the existing integer-only simulator tests remain green. |
| — | VM-032 | done ([#14295](https://github.com/adhithyan15/coding-adventures/pull/14295)) | Protect LANG native Windows execution in PR CI. | An affected `twig-aot`/LANG dependency change selects an actual Windows executable smoke run, with its toolchain present; assert real execution rather than a green Clippy-only job. Preserve platform-plan gating and keep the known GC early returns explicit. |
| — | VM-024 | done ([#14317](https://github.com/adhithyan15/coding-adventures/pull/14317)) | Protect the non-ALGOL language matrix in normal CI. | Every non-ALGOL `PROGRAMS` row executes on each available declared backend; no empty selection or silent failure-to-skip conversion; full ALGOL diagnostics remain available. |
| 2 | VM-025 | queued; coordinate with ALGOL owner | Reproduce the full matrix's current Linux failures and restore full CI coverage after their repair. | Record exact failing cells and owning PRs; remove the full-matrix exclusion only after the complete target runs green on supported CI hosts. |
| — | VM-026 | done ([#14325](https://github.com/adhithyan15/coding-adventures/pull/14325)) | Reconcile stale Twig, frontend-count, and completed-work claims in LANG-FULL and LANG-PLATFORM status documents. | Every remaining gap links a current source/test boundary; landed VM-010, VM-017, VM-018, VM-020, and VM-021 work is no longer described as missing. |
| — | VM-035 | done ([#14333](https://github.com/adhithyan15/coding-adventures/pull/14333)) | Honor Cargo target-directory overrides in shared AOT test archive discovery. | With a nondefault `CARGO_TARGET_DIR`, locate the archive Cargo actually built and execute a heap LLVM cell; never return a nonexistent guessed path. |
| — | VM-027 | done ([#14341](https://github.com/adhithyan15/coding-adventures/pull/14341)) | Audit feature-by-backend coverage for all ten wired frontends, including dedicated McCarthy and Macsyma suites. | Inventory implemented features, declared/refused backend cells, executable proofs, and CI commands; split every uncovered implemented feature into a bounded parity item. |
| 5 | VM-013 | design required | Define portable semantics for Oct's Intel-8008 intrinsics (`in`, `out`, `adc`, `sbb`, rotations, carry, parity). | Document machine state and I/O contracts, then split into PR-sized operations with both portable and real 8008-simulator proofs; resolve only essential language-design questions with the user. |
| 6 | VM-028 | queued | Audit Nib's remaining Intel-4004 arithmetic and control-flow parity beyond BCD storage. | Compare implemented operations against the 4004 backend and simulator; file bounded missing-operation or refusal tests and close them with executed proofs. |
| 7 | VM-029 | queued | Audit the broader platform vision and native-runtime roadmaps into explicit completion milestones. | Separate landed capabilities from GC, tooling, IR-bridge/transpilation, and measured-performance work; give every retained milestone a testable acceptance criterion and owner. |
| 7a | VM-031 | queued under VM-029 | Reconcile and complete native precise-GC frame-walk proofs. | Windows smoke has two explicit early-return differentials for the Rust/Twig frame boundary; inventory supported-host GC gaps, track each refusal, and execute live-byte reclamation proofs before calling native GC complete. |

## VM-027 audit discoveries and next priorities

The [feature/backend inventory](LANG-VM-FEATURE-COVERAGE.md) compares ten
frontends with the eight-language unified corpus and dedicated suites. Local
McCarthy (19 x 7) and Macsyma (21 x 7) capstones passed. No executed red cell
was discovered. Missing existing-feature proofs therefore take priority over
new machine semantics. After this audit merges, select VM-036 first: existing
native code should be exercised on Windows/Linux before expanding coverage.
VM-025 remains separately owned. The following queue precedes VM-013/028/029;
items requiring new runtime lowering follow the coverage-only promotions.

| Order | Item | Bounded work and executable acceptance |
|---|---|---|
| done #14349 | VM-036 | Run McCarthy's existing 19-program native capstone on Windows/Linux as well as macOS. Assert a present linker cannot silently skip; wire actual Windows execution into CI and prove all 19 results. |
| done #14358 | VM-037 | Promote FLOW-MATIC compare/branch/jump behavior beyond the scalar-output baseline. Use terminating, discriminating output programs on all seven standard columns. |
| done #14363 | VM-044 | Promote Oct while/loop/break and returned function values to observable seven-column programs; prove u8 wrap and actual branch/call effects. |
| done #14370 | VM-045 | Promote COBOL reference modification to standard columns: constant and dynamic bounds, result text and explicit invalid-bound behavior, compared with its existing oracle. |
| done #14394 | VM-046 | Promote COBOL STRING/UNSTRING in separate slices for SIZE, delimiters, pointer and overflow behavior; each slice needs oracle-matched output on its declared code-generation columns. |
| sliced below | VM-047 | Promote COBOL INSPECT in separate tally, replacement and region slices; preserve first-match/non-rechaining and documented character boundaries; compare executed outputs with the oracle. |
| 7 | VM-049 | Add a real .NET lane for the existing Macsyma arithmetic corpus with explicit tool gating and full result assertions; preserve the simulator floor. |
| 8 | VM-038 | Probe Macsyma v0 integer arithmetic/assignment on BEAM and add a real Erlang corpus lane, or record a precise unsupported lowering with a regression before a separate fix. |
| 9 | VM-039 | Define portable FLOW-MATIC input_more/EOF semantics, then run a finite read/process/write stream on each code-generation column; no post-detection failure-to-skip conversion. |
| 10 | VM-040 | Inventory remaining BEAM cells separately for Twig strings, Twig records/closures, Nib scalars, BASIC f64/I/O, Oct u8/I/O, FLOW-MATIC and COBOL. Each family first gets a discriminating probe; split actual lowering defects before implementation. Brainfuck remains the explicit excluded tape design. |
| 11 | VM-042 | Pin Brainfuck's intentional BEAM exclusion with a driver-level error assertion for mutable tape operations; distinguish supported frontend compilation from backend refusal. |
| 12 | VM-041 | Isolate Twig captured/reassigned runtime-string lowering from existing source-local string metadata; add one captured-string value proof before wider dynamic-string expansion. |
| 13 | VM-048 | Define a representation-neutral observation for Macsyma's implemented inert symbolic Apply, then promote one oracle-derived symbolic result per backend; do not compare raw pointer/tag identities. |

BASIC two-dimensional numeric arrays were verified in the matrix and lowerer,
so their stale README description is corrected here rather than creating a new
implementation item. The known DEF FN-global and print-zone semantics remain
future frontend design scope, not missing proofs for already-implemented code.

## Discovery log

- **VM-D026 — confirmed 2026-09-05:** source/document inspection for VM-026
  confirms ten driver variants but only eight unified corpus languages;
  McCarthy and Macsyma remain dedicated suites. Source doc comments also retain
  older claims (for example Macsyma conformance's BEAM-is-McCarthy-only note).
  The current spec reconciliation states the executed coverage boundary;
  VM-027 must include those source comments when auditing feature/backend
  declarations, rather than treating them as authoritative absence claims.

- **VM-D025 — confirmed 2026-09-05:** VM-024 validation reused a merged
  worktree's Cargo target via `CARGO_TARGET_DIR`. The shared test helper
  `tests/common::gc_core_capi_archive` builds with Cargo's inherited override but
  searches only `<workspace>/target/release`, then returns a nonexistent Unix
  archive name if neither file is there. LLVM Twig `(car (cons 42 0))` fails at
  linking for that reason. Queue VM-035 to derive the actual Cargo artifact
  path. Continue VM-024 validation with the normal target directory; this is
  an alternate build-layout helper gap, not an observed default-layout runtime
  failure and not evidence against a backend's language semantics.

- **VM-D024 — confirmed 2026-09-05:** VM-033's real multi-line/RND proof
  exposed JVM output `22`, `-914968`, `-914968`, `-398672`. A fresh process
  using the pre-VM-033 matrix binary reproduces `360:Jvm`, so normalization
  did not cause it. `concretize_scalar_any_for_jvm` narrows RND's explicit
  `i64` product to `i32` because every literal fits in i32 and BASIC's newer
  floating-point PRINT is not the old `print_i64` builtin. The second RNG
  product overflows before modulo. Promote VM-034 as VM-033's prerequisite;
  keep mixed floating-point modules out of the integer-only simulator rewrite
  and retain all three selected multi-line regressions.

- **VM-D023 — confirmed 2026-09-05:** with VM-030 applied, the proposed
  non-ALGOL matrix ran for 308 seconds before failing on LLVM Dartmouth BASIC
  multi-line real output. Expected `3.14\n.25\n-2.5`, observed
  `3.14\r\n.25\r\n-2.5`. A fresh single-cell process with
  `LANG_MATRIX_ONLY_CELL=352:Llvm` reproduces the mismatch in 0.35 seconds.
  This is a host text-output comparison failure, independent of CRT symbol
  linking. Promote VM-033 ahead of coverage work; retain the exact content and
  control-character contract rather than broadly trimming away discrepancies.

- **VM-D022 — confirmed 2026-09-05:** `.github/workflows/ci.yml` selects a
  Windows job when `needs_rust` is true, but `Build and test affected packages`
  on Windows additionally requires Swift or Elixir. Its dedicated Windows-only
  Rust step is Clippy only and does not select the portable `twig-aot` crate.
  Therefore a green Windows job for a Rust-only CRT repair does not execute
  `windows_x86_64_smoke`. Promote VM-032 as the first coverage follow-up. Local
  runs with both real linkers provide VM-030's Windows runtime evidence while
  that gap is repaired separately.

- **VM-D021 — confirmed 2026-09-05:** executing the proposed non-ALGOL
  matrix on Windows failed on the very first native Twig `42` cell with
  duplicate `__vcrt_InitializeCriticalSectionEx`, defined by both
  `libvcruntime.lib` and `vcruntime.lib`. The static library was added for an
  earlier custom `/ENTRY:main` path; that custom entry was removed by
  `bc2cb05594`, but the static library and its old explanation remained.
  Promote VM-030 ahead of VM-024. Remove the obsolete static CRT selection,
  preserve the compiler's normal startup, and run actual Windows executables
  before enabling additional matrix coverage. VM-024's draft patch is held
  outside the checkout until this prerequisite lands.

- **VM-D001 — confirmed 2026-08-27:** JVM scalar concretization narrowed
  COBOL's explicit `i64` constant `10^12` to `i32`, so nested fixed-point
  division printed `000000` instead of `000533`. Promoted to VM-001.
- **VM-D002 — confirmed 2026-08-27:** `lang-aot/BUILD` still describes the
  closure suite as red even though all five tests pass, and excludes the full
  LANG matrix. Promoted to VM-002.
- **VM-D003 — confirmed 2026-08-27:** the LANG-FULL roadmap still marks Twig
  lists/closures/records/unions and McCarthy backend completion as open despite
  landed executed tests. Promoted to VM-003.
- **VM-D004 — confirmed 2026-08-27:** comparing top-level `lang-aot/tests/*.rs`
  targets with the explicit BUILD command found `macsyma_conformance` omitted
  in addition to the two documented exclusions. Running it exposed LLVM process
  status 249 for the expected signed result -7. Promoted to VM-004 and ranked
  above CI/documentation work as a red executed conformance suite.
- **VM-D005 — confirmed 2026-08-27:** enabling `lang_matrix` in Linux CI exposed
  19 failures: LLVM links without `libm` for cross-language
  `pow`/`floor`/`trunc` calls (including Dartmouth BASIC), while native AOT
  refuses newly proven four-dimensional ALGOL array cases. The linker portion
  is promoted to VM-005; the array refusals remain with the separate ALGOL
  campaign. `lang_matrix` stays explicitly excluded from `lang-aot/BUILD`
  until both streams make it Linux-green.
- **VM-D006 — confirmed 2026-08-27:** the roadmap's E6/Twig checklist predates
  landed seven-engine proofs for list operations, records, unions, and closures,
  and still presents McCarthy backend completion as open even though W16 proves
  F1–F7 on all eight applicable backends. Promoted to VM-003. The audit narrowed
  VM-010 to the two Twig cells that still exclude VM/JIT: symbols/quote and
  forward-referenced dynamic globals (plus the documented boxed-global arithmetic
  follow-up).
- **VM-D007 — confirmed 2026-08-27:**
  `mccarthy-lisp-iir-compiler/README.md` still says L3 is the next phase even
  though the authoritative McCarthy platform matrix marks W1–W16 complete.
  Promoted to VM-015 as a separate package-documentation reconciliation.
- **VM-D008 — confirmed 2026-08-27:** the discriminating Twig forward-global
  arithmetic cell returned the tagged word for 42 (`336`, observed as process
  exit 80) on native AOT and LLVM because `lower_dynamic_arith` boxed the
  helper's result without propagating `ref<any>` through its return/call
  signature. BEAM refused the same representation-only `box` op even though
  Erlang integers are already dynamic terms. Retained inside VM-010 because
  boxed global arithmetic was an explicit part of that item.
- **VM-D009 — confirmed 2026-08-28:** VM-011 combined three independent
  implementation classes. `SIN`/`COS`/`LOG`/`EXP` already exist as shared IIR
  operations on every standard backend and need only Dartmouth frontend wiring;
  string `READ`/`DATA` needs a mixed-type ordered pool; `RND` needs an explicit
  portable RNG contract and new substrate. Decomposed into VM-016, VM-017, and
  VM-018 before selecting the deterministic VM-016 slice.
- **VM-D010 — confirmed 2026-08-28:** the Dartmouth BASIC compiler README still
  calls general exponentiation and the final NativeAOT/JVM/CLR string-array lanes
  future work even though their seven-backend matrix proofs have landed. Promoted
  to VM-019 for a focused package-documentation reconciliation.
- **VM-D011 — confirmed 2026-08-28:** the same README broadly calls richer
  dynamic string expressions future work even though the matrix executes
  `str_concat` over two runtime `INPUT` strings on all seven backends. Retained
  inside VM-019; the corrected limitation is mixed numeric/string `DATA` and
  `READ`, already tracked as VM-017.
- **VM-D012 — confirmed 2026-08-28:** VM-014 combined two different gaps.
  FLOW-MATIC is wired into `Language` and already has package-level JIT and
  backend acceptance tests but no unified matrix cell. Macsyma's separate
  conformance suite executes VM, NativeAOT, LLVM, WASM, JVM, and CLR while
  explicitly excluding the universal JIT because its callback glue is still
  McCarthy-specific. Decomposed into VM-020 and VM-021; selected VM-020 first
  because it closes a zero-coverage frontend without changing runtime semantics.
- **VM-D013 — confirmed 2026-08-28:** VM-021 did not require the anticipated
  McCarthy tagged-runtime callback generalization. Macsyma's v0 integer corpus
  becomes typed arithmetic after the existing `lower_dynamic_arith` pass, while
  generic VM/JIT `box`/`unbox` are identity operations. The dedicated
  `run_macsyma_on_jit` runner therefore adds no language-specific runtime
  callbacks; all 21 programs execute and agree across seven engines.
- **VM-D014 — confirmed 2026-08-28:** VM-017 needs one dynamically ordered
  stream without imposing a new tagged-value ABI on seven backends. Parallel
  kind, `array<f64>`, and `array<str>` pools retain one source index and shared
  `RESTORE` pointer. Every `READ` first bounds-checks the kind array and then
  validates the target type; a mismatch deliberately enters the existing
  cross-backend array-bounds trap contract instead of reading a placeholder.
- **VM-D015 — confirmed 2026-08-28:** the shared Dartmouth BASIC grammar is
  embedded in generated parser artifacts outside the Rust LANG VM frontend.
  CI caught the required Ruby regeneration; auditing the same source boundary
  found Python, Lua, and TypeScript parser artifacts with the old numeric-only
  `DATA` rule. VM-017 regenerates all four alongside Rust so no checked-in
  parser silently disagrees with the authoritative grammar.
- **VM-D016 — confirmed 2026-08-28:** the TypeScript Dartmouth BASIC parser's
  `BUILD` chains relative `cd` commands through its dependencies and finishes
  in `../parser`, so its reported 117 tests belong to the generic parser while
  the Dartmouth package's own 61-test suite never runs. Promoted to VM-022 and
  ranked above new semantics as a missing-test-protection defect.
- **VM-D017 — confirmed 2026-08-31:** VM-022's sibling audit found the same
  top-level, stateful `cd ../...` pattern in 27 TypeScript parser `BUILD`
  scripts and 22 lexer `BUILD` scripts, with 20 and 18 corresponding Windows
  scripts respectively. Promoted to VM-023 for a non-ALGOL fleet audit and an
  automated package-directory guard; it remains separate from VM-022 so the
  Dartmouth repair has a focused executed proof.
- **VM-D018 — confirmed 2026-08-31:** after excluding the separately owned
  ALGOL fronts and the already-repaired Dartmouth parser fronts, VM-023 found
  470 stateful sibling-directory commands across 81 parser/lexer build files.
  Dependency installs now run in subshells, and a CI-discovered repository
  test rejects any future top-level sibling `cd` in non-ALGOL `BUILD` or
  `BUILD_windows` fronts.
- **VM-D019 — confirmed 2026-08-31:** the 4004 simulator models 256 main RAM
  characters plus 64 status characters, while the old backend had no RAM ops
  and compiled functions independently. VM-012 uses the complete 320-nibble
  space and builds a module-wide global-slot map before lowering functions, so
  every function agrees on a static's physical address.
- **VM-D020 — confirmed 2026-08-31:** every standard backend already executes
  typed module globals, cross-function calls, exact `i64` multiplication/modulo,
  and `i64`↔`f64` conversion. VM-018 therefore needs no random host ABI: one
  Park–Miller helper can keep a module-global state shared by `main` and
  `DEF FN`. The accepted contract fixes seed 1 at program start, makes negative
  arguments reseed-and-advance, zero repeat, and positive arguments advance.

## Ownership boundary

Do not select ALGOL items from this backlog while the separate ALGOL agent is
active. Cross-cutting fixes may touch shared infrastructure used by ALGOL, but
must preserve its tests and avoid changing ALGOL semantics or roadmap ownership.
