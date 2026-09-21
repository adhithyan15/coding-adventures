---
category: Repo policy / workflow reminders
---

# A late content fix invalidates every downstream artifact, not the one you were thinking of

The human-languages corpus generates artifacts in a fixed order — figures,
books, narration, script-owner evidence, assessment artifacts, and **modality
last** — because each stage reads the output of the ones before it.

I ran the whole chain, ran the suite, and got one content failure: a lesson said
"the course" where the book must not refer to itself. I fixed the lesson, re-ran
`generate:books` because the failure had named a `.tex` file, and re-ran the
suite. Three tests were still red, now in `core/lesson-modality`.

The lesson edit had invalidated the modality manifests too. I had re-run the
stage the error message pointed at rather than the stages that depend on the
file I changed, and the error message pointed at `books` only because that is
where the *symptom* had surfaced.

**A generated-artifact pipeline has no partial re-run that is safe to guess
at.** The ordering exists because of dependencies; the moment a source file
changes, every stage downstream of it is stale, including the ones whose output
you have not looked at. A fix that touches a lesson is not "a book fix" however
the failure was worded.

So: **after any edit to a source file, re-run the entire generation chain in
its documented order, not the stage the failure named.** It takes a couple of
minutes and removes a whole class of second-round CI failure. The cheap form is
a single loop rather than a judgement call about which stages matter:

```bash
for s in figures books narration script-owner-evidence \
         assessment-artifacts modality; do npm run generate:$s; done
```

The general shape: **an error message names where a problem became visible, not
what it invalidated.** Those are the same thing only when the pipeline has one
stage.
