---
category: CI & GitHub Actions
---

# Limit fetch negotiation to main when an unrelated Codex checkpoint ref is broken

During PR #18849 monitoring, a general `git fetch origin` failed with
`fatal: bad object refs/codex/turn-diffs/checkpoints/...` and a missing-object
message. The failing ref belonged to local checkpoint tooling, outside the
delivery branch. Do not delete or rewrite another tool's refs to unblock a PR.

The following scoped fetch completed successfully in this checkout:

```sh
git -c fetch.negotiationAlgorithm=consecutive fetch --no-auto-maintenance --no-tags --negotiation-tip=refs/remotes/origin/main origin refs/heads/main:refs/remotes/origin/main
```

The live GitHub main SHA independently matched local origin/main. This avoids
the unrelated broken negotiation ref for this fetch; it does not repair the
checkpoint or establish repository-wide object integrity. Record that separate
tooling issue, preserve its refs, and verify actual fetched commits before
using them as merge evidence.
