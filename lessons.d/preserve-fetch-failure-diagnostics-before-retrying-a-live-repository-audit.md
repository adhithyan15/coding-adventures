---
category: Repo policy / workflow reminders
---

# Preserve fetch failure diagnostics before retrying a live repository audit

The CV04a heartbeat audit discarded both output streams from `git fetch`.
On 2026-10-08 at04:53 UTC the command returned exit1, leaving only a Python
CalledProcessError. A direct retry and the subsequent complete audit succeeded,
but the original cause could not be recovered. Do not label it a network error
or a shared-reference race without its actual diagnostic.

The external audit helper now captures both streams, saves a failure receipt
with UTC time/command/exit status, and surfaces stderr before stopping. Failed
fetches do not become fresh-main evidence. Retry separately, then perform the
complete main/backlog/issue/open-PR audit; retain the failed episode instead of
overwriting it with a successful retry. The active Miri source remained frozen
and its unfinished run was not relabeled successful.
