---
category: CI & GitHub Actions
---

# Rerunning a superseded head's CI run cancels the current head's run, because both share the PR's concurrency group

While babysitting several PRs, a flaky failure was handled by queuing a
background `gh run rerun <run> --failed` that fired once the run completed.
By then the PR had a newer head (a merge-conflict fix was pushed in between).
The rerun started attempt 3 of the **old** head's run. ci.yml's concurrency
group is per ref with `cancel-in-progress` on pull requests, so the newest
arrival won: it cancelled every job of the **current** head's fresh run.
`gh pr checks` then showed ~40 "failures" -- jobs with zero steps, no runner,
and unexpanded names like `Flutter platform library (${{ matrix.os }})` --
on a head with nothing wrong.

Fix: stop the stale rerun scripts and rerun the current head's run, which
supersedes the stale attempt in turn.

Next time:
- A deferred rerun must re-check the PR head first and skip if it moved:
  `[ "$(gh pr view N --json headRefOid -q .headRefOid)" = "$expected" ] && gh run rerun ...`.
- Before rerunning anything, compare the run's `headSha` with the PR's
  `headRefOid`; never rerun a superseded head.
- Zero-step cancelled jobs with no runner are a concurrency cancellation, not
  a test failure. Look at the branch's other runs before reading logs.
