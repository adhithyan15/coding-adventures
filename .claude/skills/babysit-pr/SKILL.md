---
description: >
  Watch a pull request's required checks and merge state at its exact current
  head. Investigate real failures, repair conflicts, and verify the expected
  head merged without acting on superseded or redundant runs.
user_invocable: true
---

# Babysit PR

Monitor one pull request until its current head is green and merged or until a
real blocker needs user input. Required checks on the current pull-request head
are authoritative. Duplicate push runs and runs for older heads are context,
not permission to cancel, rerun, or modify anything.

## Bind the pull request and current head

Resolve the pull request from the supplied number/URL or the current branch.
Keep the returned `headRefOid` as `expected_head` for the complete polling
cycle:

```bash
pr_json="$(gh pr view "$pr" --json number,url,state,mergedAt,mergeable,mergeStateStatus,headRefOid,mergeCommit,autoMergeRequest)"
expected_head="$(jq -r '.headRefOid' <<<"$pr_json")"
local_head="$(git rev-parse HEAD)"
```

Both hashes must be full 40-character lowercase hexadecimal values. Before a
push, the local and pull-request heads must match the state you inspected. If
the PR merged or either head changed unexpectedly, stop and re-read the PR;
never push into a closed branch or describe an older revision as current.

## Read only authoritative checks

Ask GitHub for required checks on the PR, not every check run attached to the
commit:

```bash
gh pr checks "$pr" --required --json bucket,event,link,name,state,workflow
```

Also refresh `gh pr view` on every cycle. `state`, `headRefOid`, `mergeable`,
and `mergeStateStatus` answer different questions and must be evaluated
together. An empty legacy commit-status response is not a CI result; this
repository uses check runs.

For diagnostics only, `gh run list --commit "$expected_head"` may reveal a
redundant push run. Do not use that list in place of `--required`, and never
cancel a run: the current required PR suite may still be authoritative even
when another event looks duplicative.

Classify required checks as follows:

- `pending`: wait and poll again.
- `pass` or `skipping`: healthy.
- `fail`: inspect the linked job's real conclusion and logs. Act only after a
  fresh PR read proves its head still equals `expected_head`.
- `cancel`: first refresh `headRefOid`. Ignore the result if a newer head
  superseded it. If the cancelled check belongs to the unchanged current head
  and no replacement run exists, rerun the failed/cancelled job without making
  a source commit.

Fix genuine current-head failures in focused commits. Before each push, refresh
the PR state and head again. A source change invalidates any exact-head security
approval, so repeat required local validation and mandatory security review,
then push once after batching the complete fix.

## Resolve merge conflicts without losing the head

If GitHub reports `CONFLICTING`, fetch `origin/main`, merge it into the feature
branch, resolve the conflict from both sides' intended behavior, rerun affected
validation and mandatory security review, and push normally. Do not rewrite a
shared branch merely to make the graph tidy. Re-read the PR immediately before
pushing so auto-merge cannot race a local conflict repair.

## Completion proof

Green checks are not the terminal condition. Continue until `state` is
`MERGED`, then fetch the PR one final time and verify its `headRefOid` still
equals `expected_head` and `mergeCommit.oid` is present. If local work was meant
to be included, `git rev-parse HEAD` must also equal that expected head. A
merged PR whose recorded head differs is a dropped-push incident, not success.

Report pending states quietly. Report only a real failure, conflict, unexpected
head change, successful merge, or user action requirement.
