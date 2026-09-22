---
category: CI & GitHub Actions
---

# A push while the book build is in flight cancels it and costs a red gate

Chapter 469 produced three pushes inside ten minutes: the chapter, then a
backlog entry, then a CI fix. Each push superseded the previous head, GitHub
cancelled that head's in-flight runs, and `Human Languages Books gate` failed
twice with a state that looks alarming and means nothing:

```
detect=success relevant=true build=cancelled
::error::Unexpected books gate state: relevant=true build=cancelled
```

The gate cannot tell "cancelled because superseded" from "cancelled because
broken", so it goes red either way. Neither failure was a defect, and both
arrived as CI-failure wakes that had to be read, diagnosed and dismissed.

This was the fourth such gate failure in one session. An earlier fix — run the
fact-check **before** committing, so corrections never become a second push —
removed one cause and not this one, because these extra pushes were not
corrections at all. The backlog entry went out on its own purely because it
felt cheap.

Nothing is cheap while the all-books build is running, and it runs 20 to 40
minutes. So hold every non-urgent addition — backlog shards, `lessons.d`
entries, documentation — and push them **with** the chapter: one commit series,
one push. If something genuinely must go out mid-build, expect the gate to go
red on the superseded head and do not treat that red as a signal.

The diagnostic, when a books-gate failure arrives: read `BUILD_RESULT` in the
job log and compare the failing `head_sha` against the PR's current head. If
they differ, the run was superseded and there is nothing to fix. Re-running it
is wrong — the commit it belongs to is no longer the head.
