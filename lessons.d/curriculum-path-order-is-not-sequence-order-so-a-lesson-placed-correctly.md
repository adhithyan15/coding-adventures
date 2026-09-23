---
category: Repo policy / workflow reminders
---

# Curriculum path order is not sequence order, so a lesson placed correctly by sequence can still precede its prerequisites

A human-languages lesson has **two** orderings and they are independent:

- **`sequence:`** in the frontmatter is the reading order. `measureContinuity`
  uses it, so forward references and reinforcement windows are judged on it.
- **`pathSegment` / `pathOrder`** in `curriculum-membership.d/<id>.json` is the
  curriculum-graph order. `validate` judges prerequisites on **this** one.

A Latin review lesson at `sequence: 1145` named `LA-C26-quomodo` (sequence 500)
as a prerequisite and still failed:

```
latin: LA-C26-quomodo must precede LA-C47-second-pass-honest-gaps in curriculum.json
```

`quomodo` sits on `LA-PATH-023`; the lesson had been filed on `LA-PATH-021`,
chosen from the sequence number of a neighbour. Lower sequence, later segment.

**When adding a lesson, pick its segment from where its PREREQUISITES sit, not
from the lesson next to it in sequence.** Check every prerequisite's
`pathSegment` first and take a segment at or after the latest of them.

Two related conventions, both invisible until the diff or a test shows them:

- A path attaches extensions through `before`, `inline` **or** `after`, and the
  choice is per track. Persian, Portuguese and Latin use `after`; Telugu,
  Italian and Urdu use `inline`. Read the track's existing path files, and read
  the `git diff` after editing — an append that looks like a replacement is the
  failure mode.
- Some tracks pin rules no other track has. Marwadi requires **exactly one**
  `hl-activity` per lesson and pins all of their ids as a sorted list, so a
  review lesson without one fails a test that names no rule. The corpus test
  under `tests/corpus/<track>/` is where a track's private conventions are
  written down; read it before authoring for a track you have not touched.
