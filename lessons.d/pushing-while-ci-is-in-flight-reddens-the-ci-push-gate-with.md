---
category: CI & GitHub Actions
---

# Pushing while CI is in flight reddens the CI push gate with a cancelled job, not a real failure

**Context:** PR #15839. A second commit was pushed to the branch while the
first commit's CI run was still going. Minutes later the `CI push gate`
check went red:

    ##[error]a required job did not pass (result: cancelled)
    ##[error]Process completed with exit code 1.

**What it looks like:** a required job failed, on a PR touching 971 files.
The natural next move is to go hunting through the build job for a real
breakage.

**What it is:** `ci.yml` sets

    concurrency:
      group: ${{ github.workflow }}-${{ github.ref }}
      cancel-in-progress: true

so the new push cancelled the in-flight run. The gate step treats only
`success` and `skipped` as acceptable, and `cancelled` is neither:

    case "$r" in
      success|skipped) ;;
      *) echo "::error::a required job did not pass (result: $r)"; fail=1 ;;
    esac

The env dump in the gate's log is the tell — every other job is fine and
exactly one reads `cancelled`:

    DETECT_RESULT: success
    CONTRACTS_RESULT: success
    ...
    BUILD_RESULT: cancelled

**Diagnosis, in one step:** compare the failing check's `head_sha` against
the PR's current head. If they differ, the red check belongs to a superseded
commit and GitHub has already started a fresh run on the new head. Here the
event carried `head_sha: ec50ca5aef` while the PR head was `3ab7924bc3`.

**Do not** re-run the cancelled job, comment on the PR, or start debugging
the build. The superseding push already triggered the authoritative run;
re-running a superseded one burns a runner for a result nobody reads. Check
the current head's checks instead and let them finish.

**The cost is per gated workflow, not per push.** This is the part that is
easy to underestimate. Every workflow with both `cancel-in-progress` and a
roll-up gate job contributes its own red check, so one extra push to
PR #15839 produced reds from `CI push gate`, `CI gate` and
`Human Languages Books gate` together. Two extra pushes produced four reds
across two superseded commits. Budget one red *per gated workflow* per
push.

A cancelled gate can also fail for a second reason. When the cancellation
lands early enough, the detection job it depends on is cancelled too, so
the gate trips its "detection did not succeed" branch rather than its
"unexpected state" branch:

    DETECT_RESULT: cancelled
    BOOKS_RELEVANT:
    BUILD_RESULT: cancelled
    ##[error]Human-language book change detection did not succeed.

Same cause, different error text, and the empty `BOOKS_RELEVANT` is the
giveaway that nothing ever ran.

**Avoiding it:** batch commits into a single push when the branch is already
building. On a long CI run — this repo's is broad, since a lockfile-wide
change marks nearly every TypeScript package affected — the window for
tripping this is large, so it is worth deciding *before* pushing whether the
next commit can wait. There is no free moment to push into an active run:
any push restarts the whole run, so the only real choice is how many restarts
to spend. Prefer waiting for a real signal and then pushing once.

**Generalisation:** `cancel-in-progress: true` converts "pushed again" into
a `cancelled` job result, and any gate that enumerates acceptable results
turns that into a failure. Every repo with both a concurrency-cancelling
workflow and a roll-up gate job has this behaviour; the gate is working as
written, and the fix is to read the SHA, not the job.
