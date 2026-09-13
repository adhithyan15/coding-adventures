# Auto-merge raced my follow-up push, and main landed half a change

The export-base64 PR failed CI, I pushed the fix, and the babysit loop reported
`MERGED` — with the failing check still listed. Both were true: auto-merge
squashed the branch head as it stood when the required checks went green, and
my later pushes were not in it. The run that "failed" was the only run there
ever was, for the first commit.

So `main` got the half that changes the wire format and not the half that
teaches the consumers to read it. The web smoke test on main fails, which I
confirmed by stashing the fix and running it:

```
  main as-is: FAIL apkg merge succeeds in the browser build
  with fix:   ALL PASS
```

Two things to carry:

- **A babysit loop that only watches `state` will report success for a merge
  that dropped your last commit.** It should compare the merged SHA against
  what was pushed. `MERGED` answers "did the PR close", not "did my work land".
- **`FAILED` next to `MERGED` is not a contradiction to explain away.** I
  nearly read it as a stale annotation. Checking *which SHA* the run belonged
  to is what turned it from noise into the actual finding — the run was for a
  commit two pushes behind.

The general form, which this file already has for the green direction: a status
is about a specific revision, and the revision is the part that gets dropped
when it is summarised.
