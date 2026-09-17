---
category: Repo policy / workflow reminders
---

# Adding a chapter needs five wiring files and the spine segment ledger is the one that gets forgotten

A new human-languages chapter is not one file. It is five, and a validate run
will name the four obvious ones long before you think of the fifth:

1. `<lang>/chapters.d/NNNN.json` — **pure ASCII**, including the payoff note
2. `<lang>/curriculum.d/extensions/NNNN-<ID>.json` — the lesson list
3. `<lang>/curriculum.d/path/NNNN-<ID>.json` — the same list, plus the spine node
4. `<lang>/curriculum.d/spine/NNNN-<SPINE-ID>.json` — **append the new path id to
   `segments`**
5. `core/book-generation.d/targets.d/<lang>-NNNN.json` — with
   `"scriptSet": "hindi-main"` for a non-Latin track

**The fourth is the one that bites.** Writing the extension and the path feels
like finishing the job, because both name the lessons and both mention the spine
node. The spine *ledger* is a separate file that records which paths hang off
that node, and nothing in the other four implies it. It fails as
`curriculum-segment-ledger-drift`, which reads like a file-ordering complaint
rather than "you missed a file".

Two more first-build failures arrived with it, both from writing lesson prose
before checking the parser:

- **Headings must classify.** `classifyBlock` in `src/parse.ts` matches a fixed
  set of prefixes; anything else is `schema-v2-unknown-block`, which schema v2
  rejects. Safe openings: `Warm-up`, `You'll want to know`, `Grammar Lens:`,
  `Guided Practice`, `Wrap-up Recall`, `Writing:`, `Script`, `Reading`, plus
  `… taken apart` and `Across the family`. A heading that reads beautifully and
  starts with nothing on that list stops the build.
- **Concept tags must be unique per lesson.** Giving six lessons in one chapter
  the same `concept_tag` fails as `duplicate-realization`: a concept is realized
  once, so a chapter about one topic still needs a distinct tag per lesson.

**The order that works:** write one lesson, run `validate`, fix the shape, then
write the rest. Writing six lessons and a chapter's worth of wiring before the
first validate turns one lesson's worth of mistakes into six.
