## Chapter 75 — when, and if

`ML-A1-Q-06` (*when?*), `ML-A1-JOIN-07` (*when* it happens) and `ML-A1-JOIN-08`
(*if* it happens) all close. Coverage **171/243 → 174/243 (72%)**; the joining
column **6/11 → 8/11**, the question column **8/10 → 9/10**.

| metric | before → after |
|---|---|
| exam-point coverage | 171/243 → **174/243 (72%)** |
| atoms taught | 391 → 394 |
| measurable lessons | 318 → 322 |
| `atomsNeverRevisited` | 25 → **24** |
| `forwardReferences` | unchanged (12) |
| `atomChapterSpikes` | unchanged |
| `durationViolations` | unchanged (0) |
| `payoffSurprises` | unchanged (0) |
| reinforcement-window misses | 820 → 827 |

### Three points, one piece

The three close together because they are one thing: **പോൾ**.

| | |
|---|---|
| **ഇപ്പോൾ** *ippōḷ* | now |
| **അപ്പോൾ** *appōḷ* | then |
| **എപ്പോൾ** *eppōḷ* | when? |

`ML-C50-now` handed over the top two rows and stopped. `ML-C41-deixis-system`
had already promised, in so many words, that meeting a new word in this family
means being *"taught one and work out the others."* The question form is the
missing third, and the chapter is that promise being kept.

Then the same piece turns up on a verb — **വരുമ്പോൾ**, *when he comes* — and the
conditional **വന്നാൽ** is taught beside it, because one ending is the entire
difference between the two sentences.

Verified before writing: **എപ്പോൾ**, **മ്പോൾ**, *eppol* and *mbol* each returned
**zero files** across 333 lessons, and every existing `-ാൽ` in the corpus was
lexical (പാൽ *milk*, കാൽ *leg*) or one of the two frozen words എന്നാൽ and
എന്തുകൊണ്ടെന്നാൽ.

### A ramp defect found on the way in, and repaired first

**`ML-C32-pokuka` named all three tenses and printed none of them.** Its table
gave *pōkunnu / pōyi / pōkuṁ* in romanization from end to end; extracting every
Malayalam-script token from the lesson body returned പോകുക, പോ, പോക്, ഉക and
പോയി. Across the whole corpus **പോകും and വരും each appeared zero times**, and
every script word ending in `-ും` was something else — the coordinator, or a
frozen adverb.

So a reader had been told the future ending, could say it aloud, and had never
seen a Malayalam verb in the future tense written down.

`ML-A1-V-05` was **not** mis-marked: the atom is introduced and the point is
probed. Coverage cannot see this, and `glyph-coverage` cannot either, since
every glyph in പോകും is taught elsewhere. It is not a glyph debt — it is a word
the reader can say and cannot read.

It is repaired **here rather than later**, because `-ുമ്പോൾ` is built on വരും,
and meeting വരും for the first time inside a longer subordinate form is the ramp
inverted. Filed as `HL-C393`.

### The past form is not about the past

The conditional builds on the past — വന്നു → **വന്നാൽ**, പോയി → **പോയാൽ** — and
the sentence it builds is not about the past at all. *Avan vannāl ñān pōkuṁ*:
**he has not come**, and he may never come.

The lesson slows down there, because the obvious reading is the wrong one. The
time of the sentence is carried by the verb at the end, which is the same habit
the purpose chapter pointed at from the other direction.

### What the chapter refuses to do

**It does not settle എന്നാൽ.** Chapter 73 declined to take the tail of
എന്തുകൊണ്ടെന്നാൽ apart, because grammars read it different ways. Knowing `-ആൽ`
does not change that, and `ML-C75-aal` says so on the page instead of quietly
claiming the win.

It also does not claim **പോൾ** is a word. It is not one you can use alone, and
the lesson says that rather than leaving a reader hunting for it in a sentence.

