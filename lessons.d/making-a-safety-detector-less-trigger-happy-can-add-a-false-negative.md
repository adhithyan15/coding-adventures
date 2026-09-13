# Making a safety detector *less* trigger-happy can add a false negative in exactly the direction it exists to prevent

`modality.ts` decides whether a lesson can be done while driving. Its stated bias is to
over-report `sight`, because a lesson wrongly called drivable sends a driver to a page at
speed. Issue #12665 was about the opposite failure — figurative prose ("Look at what
English built on that jar") costing lessons the driving edition — so the whole change was
a loosening, and every rule added was a new way to say "this cue does not count."

One of them, a use-versus-mention rule dropping cues inside a quoted gloss, accepted `'`
as both an opening and a closing quote mark. English contractions and possessives are
apostrophes. Any two within sixty characters forged a "gloss":

    Don't look at the chart's third bar.
       └───────── "quoted" ──────────┘     -> `look at` silently dropped

That is ordinary English, no adversary. It had already stripped a real lesson
(`ES-W00-hola-observe`) of its `sight-cue` reason and removed the narration line telling
a listener the lesson "points at something written down" — and the corpus flip count did
not move, because the lesson was `pen` for other reasons, so the damage was invisible in
every summary number I was watching.

- **When a change loosens a safety rule, the review question is not "does it still catch
  the cases I listed" but "what does it now MISS".** All my tests asserted the new drops
  were correct. None asked what else got dropped.
- **A generated artifact is a better diff than a count.** The regeneration showed
  `"sight-cue"` disappearing from a `reasons` array on a lesson whose modality was
  unchanged. Diff the reason lists, not just the labels.
- **Quote-delimiter classes must exclude the apostrophe in English prose**, and the curly
  `’` too — it is the typographic apostrophe far more often than a closing single quote.
