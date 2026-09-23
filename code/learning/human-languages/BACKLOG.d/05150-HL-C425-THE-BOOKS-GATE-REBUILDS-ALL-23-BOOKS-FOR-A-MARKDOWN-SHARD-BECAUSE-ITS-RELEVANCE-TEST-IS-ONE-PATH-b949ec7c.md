## HL-C425-b949ec7c — The books gate rebuilds all 23 books for a Markdown shard, because its relevance test is one path

**Status: OPEN.** Noticed from a CI log on #15904, then read from the workflow
rather than assumed.

`.github/workflows/human-languages-books.yml` decides whether a PR needs the
all-books build with a single diff:

```bash
git diff --quiet "$PR_BASE_SHA...$PR_HEAD_SHA" -- \
  code/learning/human-languages \
  code/packages/typescript/human-language-data \
  ...
```

Any change anywhere under `code/learning/human-languages` sets `books=true` —
and `BACKLOG.d/`, every track's `CHANGELOG.d/`, and the track `README.md`s all
live inside that tree. A commit touching only two backlog shards produced
`BOOKS_RELEVANT: true` and scheduled a **~24-minute** build of all 23 books.

Observed on a commit whose entire diff was two `BACKLOG.d/*.md` files plus one
`lessons.d/*.md` outside the tree.

### It also manufactures red checks, which is the part that actually costs something

Measured three times on this branch, on #15904 and #15905. The all-books build
takes ~24 minutes, so a documentation-only PR that is pushed to again within
that window has its build **cancelled** by the supersession — and the gate,
correctly, refuses `cancelled`:

```
detect=success relevant=true build=cancelled
##[error]Unexpected books gate state: relevant=true build=cancelled
```

Had the change been correctly irrelevant, the same push would have taken the
gate's other legitimate branch, `false:skipped`, and passed. So the
over-trigger converts an ordinary supersession into a **red check on a
documentation commit** — three times here, each needing a log read to confirm it
was not a defect.

**Correction, measured after writing the paragraph above.** That paragraph
credited the over-trigger with manufacturing the red checks. It is true of the
books gate and **false in general**. Reading the other two reds on the same
superseded head:

```
CI gate:       detect=success contracts=cancelled … build=cancelled
CI push gate:  detect=success contracts=cancelled … build=success
```

`build` and `Repo-wide metadata contracts` run on **every** PR, book-relevant or
not, so `CI gate` and `CI push gate` go red on any supersession whatever this
entry's fix does. Of the five reds observed today across #15904 and #15905, only
the two `Human Languages Books gate` ones are attributable here.

So the honest cost of the over-trigger is: ~24 CI minutes per documentation PR,
plus **one** of the three red gates a supersession produces. The other two are a
separate and more general question — whether an aggregate gate should report
`cancelled` as failure on a run its own successor replaced — and that belongs in
its own entry if anyone takes it up, not folded in here.

A gate that goes red for a reason unrelated to the diff is still a gate people
learn to skim past; this entry simply cannot claim to fix most of it.

### The bias is the right way round, which is why this is low priority

Over-building is the fail-safe direction. A relevance test that misses a real
book input ships a stale book; one that fires too often costs CI minutes. Any
narrowing has to keep that asymmetry — so this is a cost question, not a
correctness one, and it should not be "fixed" by a quick path exclusion that
someone later has to debug when a book goes stale.

If it is taken up, the shape that preserves the bias is an **explicit exclude
list** for paths that provably cannot reach a book — `BACKLOG.d/`,
`*/CHANGELOG.d/` — rather than an allow-list of book inputs, because the
allow-list is the thing that would silently go stale. Note the same file already
carries two hand-kept lists (`doc_shard_globs`, `tracked_doc_monoliths`) whose
comments record that **both had already drifted**, each missing Hindi; a third
hand-kept list in the same file would be the third to drift.

### Not a defect in the deletion guard

Worth stating because it is easy to misread: the `doc_shard_globs` list a few
lines above is an **append-only deletion guard**, not an ignore list. It fails
the build when a history shard is removed. It has nothing to do with
books-relevance, and reading it as an exclusion is how this entry nearly got
written with the wrong cause.
