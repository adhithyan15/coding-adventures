# Complete the Dependabot sweep

The authenticated GitHub alerts API reports 409 open npm alerts in 330 manifests
on 2026-10-11. PR #17185 already carries most compatible lockfile updates. Finish
that PR against current main, then verify GitHub's actual alert state after merge.

## Remaining dependency paths

- Replace Jest in trig, wave, matrix, gradient-descent and loss-functions with
  Vitest 4.1.11. Preserve every numerical test and the optional native matrix
  parity suite. Remove Jest's Istanbul / js-yaml 3 / argparse 1 / sprintf-js path.
- Override Electron packaging's optional global-agent with 4.1.3, whose dependency
  graph removes roarr and sprintf-js. Verify @electron/get's proxy bootstrap API.
- Generated Electron projects must emit the same override and pin shell-quote to
  1.11.0, the patched release published 2026-09-29. This avoids the newer
  concurrently release without bypassing Engram's seven-day resolution floor.
- Resolve http-cache-semantics 4.3.0 under the existing release-age floor.
- Update barcode-layout-1d's lockfile, rerun its conformance tests and record the
  resulting package tree and verified revision in its fixture evidence. Merge
  this PR with a merge commit so the verified revision remains an ancestor.

## Validation and landing

Compare every alert's vulnerable range with its manifest's resolved versions;
audit all tracked npm locks for advisories that GitHub has not yet indexed. Run
the changed test suites, TypeScript builds, generated-lock contract tests and
fixture registry tests. Review registry URLs, integrity hashes, new install
scripts and override compatibility before pushing. Keep required CI checks and
CodeQL green before merging; refresh alerts after GitHub processes the new main.

CI on the previous head failed on human-language-data corpus test timeouts and a
Windows sandbox launch deadline. Reproduce and fix the underlying constraints,
preserving coverage and sandbox enforcement.

The Windows JVM writer must reject NTFS reparse points using lstat file
attributes on supported Python versions, including Python 3.10. Check existing
root ancestors before canonicalization and each output component before writing.
Regressions cover a junction root, missing descendants and existing descendants.
Corpus assertions may share one immutable measurement per file; invalid mock
levels must fail before any corpus read. Preserve live CLI and baseline checks.

The overlapping Windows Node ACL test uses one owned copy of the runtime for
both launches. Use the production five-second startup budget and a five-second
child lifetime for the overlap fixture, matching the Python overlap test, while
retaining its 15-second case deadline and both exit assertions. On run
38163286318 the owned Node fixture took 2.3 seconds and was cancelled at its
test-only two-second attestation limit; the earlier DACL privilege failure was
already fixed. System-wide runtime ACL writes require unrelated privileges.

Load the immutable modality corpus and build its manifest once during module
initialization, as the other real-corpus fixtures do. The same CI run passed
207 human-language test files but exceeded the 30-second modality setup hook.
Keep the actual assertions and CLI checks, using the explicit budgets below.

The default-config local modality rerun exposed repeated immutable artifact reads
inside two assertions (122.6 seconds and 97.6 seconds). Load the real committed
manifest once alongside the real lessons, retaining the live `--check` invocation
and all equality checks. Use directory junctions for the directory-link rejection
fixtures on Windows so non-administrator validation exercises the same boundaries.

The remaining live CLI check measured 74.2 seconds after those repeated test reads
were removed. Each CLI invocation currently builds the identical modality manifest
twice: once for validation and once for emitted owner bytes. Build it once and
derive owner bytes from that same manifest. Preserve all live byte comparisons,
language/lesson/narration identity checks, owner reconstruction and write recovery.
After eliminating the duplicate build, the live CLI regression still measured
66.1 seconds against its old 60-second budget. Give this single full-corpus,
direct-owner filesystem integration case an explicit 120-second budget. Ordinary
test and hook limits remain 30 seconds. Do not bypass owner/path validation or
move this live CLI call into unbounded module setup to avoid its deadline.

Run 38176850343 built 532 macOS packages but failed human-language-data: three
chapter snapshot assertions exceeded 30 seconds, and the figure-target worker
exhausted its 2 GiB Node heap during shard registration. The chapter tests load
the same immutable corpus four times and calculate the same gate report three
times. Share one corpus, chapter ledger, policy and gate report within that test
file, preserving all snapshot, clean-track, pattern and sequence assertions.

Five real-corpus groups across four figure-target case shards independently
retain the full parsed corpus and resolve its filmstrip targets. Give those
groups one shared, immutable fixture
containing the lessons, resolved filmstrip targets and lookup collections. Keep
synthetic fixtures independent and preserve every discovered shard and assertion.
Validate both suites with the normal test configuration and a 2 GiB worker heap;
do not increase heap limits, raise ordinary deadlines or skip cases to hide the
failure. Refresh main before implementing, then review and run CI on the new head.

Run 38190146667 built 533 Windows packages successfully but failed one
script-ductus ownership-pin assertion. Its fixture writes both `kannada.json`
and `Kannada.json`; NTFS replaces the first file's bytes while preserving its
lowercase directory entry, so no unsafe name reaches the loader. Construct the
uppercase case by removing the lowercase map entry before writing any files.
Assert each malformed fixture's exact on-disk filename inventory before testing
loader rejection. Preserve all malformed-content and file-symlink checks, the
loader's lowercase-only policy and every ownership baseline assertion. Validate
the case fixture directly on Windows and rerun the existing ownership suite;
record any unavailable local file-symlink privilege separately from case checks.

The same run completed macOS with six 30-second timeouts in letter-ledger,
levels, verbs and track-progress tests. Reuse the existing real corpus's script
inventory in letter-ledger; load levels' immutable corpus once and calculate its
summary once; share verbs' immutable coverage report between snapshot assertions.
Keep every existing assertion and independent synthetic fixture. The two progress
cases must still invoke the real `--check` and `--report` CLI inside their test
bodies, with an explicit 120-second full-corpus integration budget. Do not cache
production loader results or raise ordinary deadlines, worker counts or heap
limits. Validate these four suites together under the normal configuration and
a 2 GiB heap cap, and record the measured CLI costs.

Main subsequently merged two Dependabot PRs that raise four Electron app ranges
from 43.4.0 to 43.5.0, conflicting with this branch's patched 43.7.9 resolutions.
Merge those main changes without restoring older resolved packages. Raise the
four declared Electron minimums to 43.7.9, matching their already-reviewed locks,
and preserve global-agent 4.1.3. Regenerate only the root manifest entries; no
package version, registry URL or integrity hash should change. Recheck every
current alert and audit these reconciled locks before reviewing the new head.

Preserve the concurrent browser warm-up repair, but restore static preflight
before any Chrome launch. Give retirement failures an internal typed identity
that is checked before log truncation, and wrap native kill/cleanup errors too.
Keep the same three recorded samples and 0.95 performance threshold. Regressions
must cover oversized preflight artifacts, long startup errors hiding retirement
text, and native cleanup errors whose messages do not name retirement.

The 8d472b2d macOS build passed 209 human-language test files and failed only
standalone-book's frontmatter assertion, which reparses the same corpus already
used by its module fixture. Reuse that lessons fixture. Audit all remaining
default-root snapshot reads in the package: chapter hashes, exam inventories,
literal markup, level gates, gentle-ramp and German/Spanish reading-task shapes.
Share immutable parsed inputs per file; preserve freshly derived mutable reports,
synthetic counterfactuals, every assertion and existing deadline. Leave live CLI
calls, explicit temporary-root loader regressions and production loader behavior
unchanged. Validate the complete package with its ordinary configuration, coverage
threshold and a 2 GiB heap cap before pushing, rather than repairing these same
readonly parsing costs one cold CI case at a time.

The full local two-worker coverage run also measured a single real-corpus
cousins snapshot at 59.3 seconds, with its default-root parse still inside the
30-second assertion. Move only that immutable input parse into the module
fixture; retain both cousin indexes, joins and every assertion inside the
ordinary deadline. The audit must cover single expensive fixture reads as well
as repeated reads.

The per-track curriculum membership snapshot likewise measured 36.5 seconds
while loading the real default-root curriculum graph inside its 30-second case.
Initialize only that immutable graph once at module scope. Keep digest creation,
pin inventory/equality assertions and the explicit pin-update branch inside the
case, with all temporary-root ownership regressions independent and live.

Persian's real writing-ladder snapshot measured 64.8 seconds around another
default-root corpus read. Share one parsed fixture in `tests/corpus/real-corpus.ts`
with that case and the seven Kannada/Tamil/Telugu/Sanskrit exam coverage/probe
cases that likewise read the immutable default corpus. Keep writing measurements,
exam inventory reads, fresh coverage sets/reports and every assertion inside
their original deadlines. Do not change language-continuity helpers, synthetic
fixtures, input validation or test discovery. Verify under the same 2 GiB heap.

Seven live full-corpus CLI checks exceeded their existing integration limits:
the three Spanish report checks measured 60.5-72.3 seconds, narration measured
71.7 seconds, and the three all-ledger shard checks measured 31.9-32.7 seconds.
Confirm the completed run's diagnostics identify timeout-only failures before
changing those cases. Keep each CLI invocation and exact report/exit comparison
inside its test body. Give Spanish/narration checks explicit 120-second limits
and all-ledger checks explicit 60-second limits. Do not replace live reads with
cached reports, narrow the production loaders, alter ordinary 30-second defaults,
change workers/heap/coverage or skip path-security cases. Revalidate these files
with the normal two-worker configuration and 2 GiB heap cap; separately report
the host's unavailable file-symlink privileges.

## Recover the macOS result receipt, 2026-10-12

At source head 977b3bfa79f332c2a0a9dadc5d99bccf5628450b, macOS job
114812563161 reached `5325/5325 done` after 5512.4 seconds at 15:45:34 UTC.
The Go 1.26.1 darwin/arm64 build tool then reported `SIGBUS` and `panic during
panic` before the package result table and failure exit decision. The runner
remained open until cancellation and reported `Illegal byte sequence` while
handling its step summary and diagnostic log. The exact cause is not established.
The progress count includes every package state and is not a passing receipt.

Preserve the cancelled log and record the failure lesson. Run a fresh hosted
attempt with the same reviewed package source, assertions, two workers, 2 GiB
heap, coverage thresholds and case deadlines. Do not change production code or
resource limits solely from this incomplete native traceback. Require an actual
zero-failure macOS result table and terminal success for the protected CI and
Books gates before landing. If the native crash recurs, use its exact phase and
retained diagnostics to guide a focused investigation instead of repeatedly
restarting without evidence.

Windows and Linux each produced a complete 535-built / 4790-skipped / 5325-total
zero-failure table on that source head. Both CodeQL analyses, all three Forme
host gates, the Forme verdict and the Books gate also passed. Preserve those
source receipts while distinguishing the fresh final-head checks from the
cancelled macOS attempt. This recovery note changes documentation only.

## Reconcile the shared parity ledger with current main

Merge origin/main at 92f7d0f5e68687bda9c97e989bf9278578b36361 while preserving
its new Arc2D evidence and every automatic non-conflicting change. The only
manual conflict is the barcode existing-lanes record. Retain the security
refresh implementation/validation revision 50ae33c4994d60eb79aaad33bc0318f56d8dc444
and its validation-note prefix. Preserve main's corrected historical merged head
f12ad9eca4532044feafe8c4ac12ad6842cd0dd2 and detailed original merge notes; GitHub
PR #16343 confirms that head and merge commit 2a75d68df071c01cbe9cd76d791904afb19819a5.
Reject duplicate JSON keys or residual conflict markers, then rerun the barcode
fixture registry validation. Main introduces no npm manifest or lockfile changes
relative to the previously merged b1f4e45913. The fresh final-head CI must validate
the reconciled tree; earlier source receipts do not replace it.

## Keep Flutter cache cleanup bounded, 2026-10-12

At b3abbdefa44c0ea77b3a2b6ec8a481eaa7bfbb44, macOS job 114991799640
produced a complete result table: 535 built, 4,790 skipped, 5,325 total, and no
failed packages. The job nevertheless failed in `Post Set up Flutter` because
the pinned action's `hashFiles('**/pubspec.lock')` exceeded its 120-second limit
after builds populated the checkout. This is a separate, diagnosed cache
cleanup failure; it does not establish the cause of the earlier Go SIGBUS.

The pinned Flutter action always appends that recursive hash to its pub cache
key, so overriding only `pub-cache-key` cannot fix the traversal. Explicitly
disable the action's optional pub cache in the general build job, retaining
`cache: true` for the Flutter SDK. Let Dart pub resolve and download dependencies
normally. Preserve all production code, runtime gates, package assertions,
coverage thresholds, workers, heap limits and deadlines. Existing Flutter and
Swift runtime workflow acceptance checks must pass, and the next hosted job must
complete successfully through its post-job cleanup.

The repair also incorporates origin/main at
3774ebb672b6468a7304229f9c61a22ba3303265 without manual conflicts. Main has no
npm manifest or lock changes since 92f7d0f5e68687bda9c97e989bf9278578b36361;
the reviewed dependency graph and tested barcode ancestor remain intact. Fresh
final-head CI must validate this combined tree before requesting merge approval.
