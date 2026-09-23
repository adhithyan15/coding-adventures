---
category: Repo policy / workflow reminders
---

# The ladder's worst blocker is not the whole cost, and the obvious fix can raise another

The new per-track ladder prints one blocker per track — the worst one:

```
telugu   touches A2  complete none  working pre-A1  vocabulary 221/300 — vocabulary short 79
```

Read alone, that says "author 79 headwords", and I was one step from opening
exactly that tranche. The track's full blocker list says three things, not one:
`vocabulary short 79`, `reinforcement short 50`, `atom-budget short 1`. HL09
§3.1 is a **conjunction** — all of them have to clear.

**Worse, the obvious fix raises the second one.** Every new content lesson
introduces at least one atom, and criterion 4 wants every atom at or below the
level revisited at least twice. Seventy-nine new headwords take the
reinforcement debt from 50 atoms to roughly 130. The tranche would drive the
headline number to zero and leave the track *further* from the rung than it
started, by the gate's own arithmetic.

**Read the whole blocker list before costing work, and check whether the fix
for one criterion feeds another.** A summary that shows the maximum is showing
you where to look, not what to do. The fix here is to plan the vocabulary and
its reinforcement together — `practice` and `review` lessons are excluded from
`CONTENT_TYPES`, so they discharge reinforcement without inflating the headword
count or the atom budget, which is the fact the whole tranche has to be built
around.

Same shape as HL-C421, one level up: a real number, correctly computed, read as
the whole answer.
