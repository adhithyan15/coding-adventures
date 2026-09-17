## HL-C397 — the books gate reports a cancelled build as a failure, so every superseding push goes red

Found while babysitting the Malayalam chapter PRs. It costs nothing in
correctness and a great deal in attention, which is the expensive kind of bug
in a gate.

### What the gate does

The "Human Languages Books gate" job reduces two upstream results to a verdict:

```
detect=success relevant=true build=cancelled
::error::Unexpected books gate state: relevant=true build=cancelled
```

Its `case` arm accepts exactly two states — `true:success` and
`false:skipped` — and everything else falls to `*)` and exits 1. **`cancelled`
lands in that catch-all.**

### Why that fires constantly

The all-books build is the long pole in this repo's CI. Any push that lands
while one is in flight cancels it, and the gate on the **superseded** SHA then
goes red. The push that caused it is usually the author's own next commit.

Observed so far, all of them spurious: **#15462**, **#15478**, and **twice on
#15479**. In every case the current head was building fine and the red was on a
SHA nobody was waiting for.

### Why it matters more than it looks

A gate that is red for reasons unrelated to the change is a gate people learn to
skim. The repo's own lesson file already says a version of this: *"a gate that
fails on recorded debt teaches authors to route around it."* This is the same
failure mode arriving from the other direction — the gate is not wrong about the
corpus, it is wrong about CI mechanics — and the habit it trains is identical.

It also costs real time on every PR: each red check has to be opened and read
before it can be dismissed, and the only way to tell a cancellation from a
genuine books failure is to fetch the job log and look for `build=cancelled`.

### The fix

Add a `cancelled` arm that does not fail:

```
true:cancelled)
  echo "The all-books build was cancelled, most likely superseded by a newer push."
  exit 0
  ;;
```

`exit 0` is right rather than generous. A cancelled build on a superseded SHA is
**not evidence of anything**, and the SHA that superseded it runs its own gate.
Concurrency cancellation is the intended behaviour of the workflow, so the gate
should treat it as a non-event, not as a fault.

Worth checking whether the neighbouring "CI gate" and "CI push gate" rollups
share the pattern — the events on #15478 and #15479 suggest at least "CI push
gate" does.

### What this is not

Not a reason to stop reading red checks. A books gate that reports
`relevant=true build=failure` is a real failure and stays one. The change is
narrow: one state, currently conflated with failure, is separated out.
