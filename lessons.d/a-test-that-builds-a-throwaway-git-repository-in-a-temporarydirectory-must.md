---
category: CI & GitHub Actions
---

# A test that builds a throwaway git repository in a TemporaryDirectory must turn off git's auto gc and maintenance

**What went wrong.** The "Repo-wide metadata contracts" step failed on #18346, a PR that does not touch the test, with
`OSError: [Errno 39] Directory not empty: '.git'` in
`test_mosaic_compose_runtime_ci_acceptance.test_a_lane_script_change_alone_requires_acceptance`.
The test creates a git repository inside `tempfile.TemporaryDirectory()` and
makes a few commits. After a commit, git may start a *detached* `gc --auto` /
`maintenance run --auto` process, which can still be writing a lock or log file
in `.git` when the `with` block ends. `TemporaryDirectory` then finds `.git`
non-empty halfway through `rmtree` and raises. Whether it happens depends on
timing, so the failure lands on whatever PR happens to be running.

**The fix.** Pass `-c gc.auto=0 -c maintenance.auto=false` in the test's `git`
helper. The same helper exists in `test_mosaic_swift_runtime_ci_acceptance.py`,
and both are fixed.

**What to do differently.** Any test that runs `git commit` in a throwaway
repository must disable auto gc and maintenance in its git helper. Grep for
`TemporaryDirectory` near `"git"` when you add one.
