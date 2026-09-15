---
category: Testing & coverage
---

# A test that reads a generated aggregate passes locally and ENOENTs on CI

2026-09-15.

Two tests read a changelog with `readFileSync(path)`. A later PR in the same
arc made that file a **generated, gitignored aggregate** — the shards became the
source of truth and the monolith stopped being committed.

Both tests then died in CI:

    Error: ENOENT: no such file or directory, open
      '.../code/specs/data/adj-facts-stdlib/CHANGELOG.md'

They passed on my machine, every time, because an earlier `--unshard` had left a
rendered copy in my working tree. **My tree had a file CI's clean checkout did
not.** Nothing in the test named that dependency, and nothing local could
surface it, because the thing that made it pass was invisible: an untracked,
ignored file that `git status` does not mention.

The fix is to read what is COMMITTED. Every other real-document test in that
file already did — `unshardDocContents(root, plan)` reads the shards and
rebuilds the document, so it cannot go stale against them and does not care
whether anyone has rendered the aggregate.

**How to apply.** Before trusting a green local run of a test that touches a
generated artefact, delete the artefact and run again. More generally, when a
migration turns a committed file into a generated one, grep for readers of that
path — the readers are the part the migration does not automatically fix, and a
gitignored leftover will hide every one of them from you.

Note the asymmetry that makes this nasty: an ignored file cannot be noticed by
inspecting the repository, only by removing it. `git status` is silent, a diff
shows nothing, and the test output says PASS.

Related: verify the way the consumer runs it; a stale pin that agrees merges
silently.
