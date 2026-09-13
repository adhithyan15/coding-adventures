# Stop waiting for CI's truncated logs to dribble out failures one round at a time — run the build tool's own detection locally against the same diff base

Four consecutive CI rounds on PR #11553 each fixed a batch of Windows failures
only to have the NEXT push reveal another batch — 21, then 17, then 16 (13
identifiable). Two things made this slower than it needed to be:

1. **`get_job_logs` truncates to roughly the last 5,000 lines of a job's
   combined output, regardless of the `tail_lines` value requested** (100,000
   and 2,000,000 both returned the identical last-~564KB slice). For a
   22,784-line "build and test affected packages" step, that's the last
   ~22% only — the earlier ~78% (including some `--- FAILED: pkg ---`
   markers) is simply not retrievable through that tool, and there's no
   `offset` parameter to page through it. Downloading the raw log directly
   (the Azure blob SAS URL `get_job_logs` returns with `return_content=false`)
   is also not an option here — the sandbox's outbound proxy denies that
   specific blob-storage host outright (403 on CONNECT), a policy decision,
   not a transient failure.
2. **The fix doesn't require CI at all.** The build tool that decides what's
   "windows-affected" runs anywhere: `go build -o /tmp/build-tool-local .`
   in `code/programs/go/build-tool/`, then
   `./build-tool-local -root . -diff-base <same base CI diffs against> -dry-run
   -emit-plan plan.json -validate-build-files=false`. The emitted plan JSON's
   `platform_overrides.windows.affected_packages` is the EXACT list windows-latest
   CI is about to attempt — computed from git diff + BUILD graph, not from
   actually running anything, so it doesn't need Windows or even a full build.
   Cross-referencing every package's resolved `BUILD_windows`/`BUILD` content
   against the two known bug regexes (`"\.\[dev\]"`, `\$\(uname\)`/`set -eu`)
   found 23 more failures in one pass — MORE than the partial CI log could
   even show, comprehensively, without waiting ~80 minutes for a Windows
   runner and then only seeing the tail of the result.

**Lesson:** once a bug's regex/pattern signature is known (from even one
confirmed CI hit), don't wait for CI to reveal the rest of its instances one
truncated log at a time — grep for the pattern across whatever the build
tool's own "affected on this platform" computation says is in scope, using
the SAME diff base CI will use. This is strictly more complete than the log
(reaches packages CI hasn't logged yet) and doesn't burn an ~80-minute round
trip per increment. Reserve actual CI runs for what can't be predicted from
static analysis: real test failures, runtime-only platform gaps (permission
enforcement, ABI-dependent stack sizes), and confirming the fix set is
actually exhaustive.
