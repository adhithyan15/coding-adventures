## Fixed — chapter 1's payoff covers the whole chapter

Chapter 1's payoff was HI-W02-ka-ta-mouth-order, a writing lesson that sits
before the chapter's closing practice, and it assessed only its own three
atoms: 3 of the 15 the chapter introduces (0.20), below the 0.5
`chapter-payoff-not-representative` floor. The payoff now moves to
HI-C01-practice, which is the chapter's real last lesson (sequence 100, after
the writing ramp at 60-90), and it assesses all 15 (1.00). The practice already
exercised the five greeting words, their two heritages and its own practice
atom; a new "Guided Practice — the first letters, by hand" section adds a
`[YOU WRITE: …]` cue for न, म, क and त (body first, bar last, then compare and repair
one) and four spoken recall prompts drawn only from what the writing lessons
taught: the shirorekhā ("head-line", usually drawn last as one line), the
shared right-spine frame (म starts at the top; क opens with a loop, त with a
curl), the inherent vowel and the Ge'ez-named abugida, and the stop families
ordered by where the mouth closes. The lesson now lists
HI-W02-ka-ta-mouth-order as a prerequisite (so the nine writing atoms are
closed), adds them to `requires.knowledge` and `practises.knowledge`, declares
the `writing` skill, and raises `duration.max_seconds` from 240 to 280 to cover
the computed 276 s. `chapters.d/0001.json` gets a refreshed summary and a note.
