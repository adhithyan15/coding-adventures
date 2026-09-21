---
category: CI & GitHub Actions
---

# pull_request_read get_status reports commit statuses, not check runs, so it reads 'pending' forever on a repo whose gates are all check runs

Waiting on PR #15844 I polled `pull_request_read` with `method: "get_status"`
eight times over roughly an hour. Every call returned:

```json
{"state":"pending","sha":"33ccd8…","total_count":0,"statuses":[]}
```

The PR had **already merged**, at 11:00. `method: "get"` said so plainly:
`"state":"closed"`, `"merged":true`, `"merged_at":"2026-09-21T11:00:07Z"`.

`total_count: 0` with an empty `statuses` array was the tell, and I read past it
every time. It does not mean "nothing has reported yet". It means **this commit
has no legacy commit statuses at all** — and it never will, because every gate
in this repository is a GitHub Actions *check run*, which is a different API.
The endpoint was answering a question I was not asking, and answering it
correctly.

**Use `method: "get"` and read `merged` / `state`.** That is the authoritative
signal for whether a PR is done. `get_status` is only meaningful where something
actually posts commit statuses.

Two related traps met the same day:

- `actions_list` with `branch:` returned runs for an entirely different branch.
  Filter on `head_sha` yourself, or query the PR.
- The webhook `check_suite.completed` notice explicitly says it does not cover
  "this App's own suites and legacy commit statuses" and tells you to verify the
  PR's overall state before acting. That warning is the same fact from the other
  direction.

The cost was not just wasted calls: it was an hour of believing the PR was still
building while the branch could have been reset and the next change started. A
status endpoint returning an empty collection is ambiguous between *not yet* and
*not applicable*, and the cheap disambiguation is to ask the object itself.
