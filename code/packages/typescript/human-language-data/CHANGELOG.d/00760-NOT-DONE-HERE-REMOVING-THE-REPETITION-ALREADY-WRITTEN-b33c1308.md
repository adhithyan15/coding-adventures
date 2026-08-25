### Not done here: removing the repetition already written

The prose still re-states etymology in Guided Practice and Wrap-up Recall blocks. An
automated sweep over chapters 1–3 was attempted and **reverted** — review found it
corrupted a URL and an `i.e.` by inserting spaces inside them, deleted 23 load-bearing
`[PAUSE 3s]` cues that `explicitPauseSeconds` and the narration script both consume,
glued four headings, left dangling connectives ("Two things:" above one question), took
eleven questions that were testing real skills, and in about ten files removed the
lesson's single *teaching* of an etymology while keeping the *drill* — the exact
inverse of the directive.

Two lessons from that: in Arabic and the other Semitic tracks **"root" means the
three-consonant root system**, which is core grammar and not an etymology hook, so a
matcher keyed on the word cannot be trusted; and a recall paragraph cannot be re-flowed
freely, because `countPromptLines` counts lines containing a question mark and a
re-wrap therefore changes the computed duration.

That work needs to be done by hand, per track, and is not attempted here.

