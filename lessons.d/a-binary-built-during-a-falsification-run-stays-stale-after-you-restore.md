---
category: Testing & coverage
---

# A binary built during a falsification run stays stale after you restore the source, so the next measurement describes the reverted code

Falsifying a fix means reverting only that change, rebuilding, and watching the
guard fail. The rebuild is the point. What is easy to miss is that the rebuild
leaves a binary on disk containing the REVERTED code, and `git checkout HEAD --
<file>` restores the source without touching it.

In #15275 the next measurement after a falsification generated QML with that
stale binary and reported `radius: 4` -- the pre-fix output -- as the state of
the branch. It then looked as though an unrelated later change had "enabled"
the conditional radius, which is a fiction: the binary had simply caught up.

The tell is a measurement that moves when you change something that cannot
affect it. Treat that as a stale-artifact hypothesis before an interesting one.

**What to do**

- Rebuild explicitly after restoring the source, in the same command as the
  restore, so the two cannot drift.
- Better, make before/after measurements symmetric: build, measure, revert,
  build, measure, restore, **build**. Every measurement gets its own build.
- Never reuse a baseline directory generated at an unknown point in the
  session. Regenerate both sides when it matters.

Related: a measurement must come from the emitted artifact, and an artifact is
only as current as the binary that emitted it.
