## HL-C355 — Urdu's numerals cost NO new letter, and the teens are where Urdu and Persian part company

**HL-C350's third zero-numeral track, closed, and the two closed on the same day
disagree about the teens in a way worth recording.** Urdu taught no numeral
across 134 lessons: **ایک** and **دو** both returned zero occurrences as words —
the two raw matches for **دو** were inside **دوست**, "friend" — so a track with
fourteen verbs and a repair kit could not count to one. Chapters 28–32 add 25
lessons and close `UR-A1-Q-01`, `UR-A1-Q-02`, `UR-A1-Q-03` and `UR-A1-FUN-53`;
coverage moves 130/234 (56%) to 134/234 (57%).

**IT COST NO NEW LETTER, AND THAT WAS MEASURED BEFORE ANYTHING WAS DESIGNED.** A
census of the track's 38 Perso-Arabic characters showed every sign in all twelve
number words and all four ordinals already on the page. **The largest lexical
gap in the file was among the cheapest to close** — which is the third time in
this campaign that reading the corpus rather than the coverage column changed
the size of the job (Marwadi needed one letter for twelve numbers; Gujarati's
apparent script debt was ten of eleven devices already writable).

**THE FINDING THAT ONLY APPEARS WHEN TWO TRACKS SHIP TOGETHER: the teens are not
one problem.** Persian builds every number from eleven to nineteen by putting a
unit in front of *dah*, so HL-C354 taught three and had the reader BUILD the
other six — one chapter, one rule. **Urdu does not.** *gyārah, bārah, terah,
chaudah, pandrah, solah, satrah, aṭhārah, unnīs* wear the ten down differently
in almost every one; they are nine words, and so is every number up to
ninety-nine. So Urdu's teens are deliberately NOT in this tranche, chapter 30
says so to the reader in as many words, and `UR-A1-Q-01`'s probe names
`UR-GRAMMAR-TEENS-IRREGULAR` rather than nine lexical atoms it would otherwise
be inventing. Two sibling scripts, one shared alphabet, and opposite answers to
"is this a rule or a word list" — a per-track reading finds neither.

**THE NUMERALS ARE WHERE URDU DID NOT BORROW FROM PERSIAN**, and the track's own
two-roads thread finally has a clean case. *garmī* and *sardī* came in from
Persian; Persian's *yek* was as available as Persian's *sard*, and Urdu declined
it. The proof is *sāt* against Persian *haft*: Iranian turned every word-initial
*s* into an *h* and Indo-Aryan did not, so a word that came in FROM Persian
carries the change and an inherited word does not.

**A LATENT INVENTORY BUG, found by trying to write a headword.** The
Perso-Arabic maddah's carrier evidence lived under a singular `example` key
while `uncoveredGlyphs` — and the mark file next to it — reads `examples`. The
composition was source-verified and invisible to every consumer, so **آ** read
as an uncovered glyph with its own evidence sitting in the same file, and
**آٹھ**, eight, was unwritable for a reason that was a typo. Converted to the
array form. **The general shape: a schema that accepts two spellings of one
field will eventually be written in the one nothing reads**, and the failure is
silent in the flattering direction — the glyph-gap queue stayed empty because
the glyph was never asked for.

**THE DIGITS ARE A READING GAP, AND THE INVENTORY HAD ALREADY SAID SO.**
`UR-A1-Q-02`'s old note observed that Urdu draws four and six differently from
Arabic "even though both scripts share an alphabet — a fact the corpus's own
cross-script comparisons would make cheap to teach." Chapter 31 is that note
carried out: ten signs in four lessons grouped by the reversed DIRECTION (a
number runs left to right inside a right-to-left line), the one-tooth pair 2/3,
the three Arabic draws differently, and the 7/8 mirror. **A note written as an
instruction gets executed; a note written as a lament does not.**

**REINFORCEMENT, DECOMPOSED.** 277 → 344, and **none of it is this tranche's**:
every atom it introduces meets R1, R2 and R3, verified atom by atom. The +67 is
pre-existing debt that growing the track from 134 to 159 lessons made judgeable
for the first time, R4 alone going 96 → 143. Script-closure violations (4),
never-taught glyphs (20), atoms never revisited (3), atom spikes, payoff
surprises and duration violations are all unchanged. One forward reference IS
created, 3 → 4, and it is an observation rather than a defect: teaching **چھ**,
six, makes the earlier emphasised **چھ** inside **پوچھنا**, "to ask", a forward
reference, because that syllable is the number.

**A COLLISION WORTH RECORDING, because it is a workflow finding rather than a
content one.** This tranche was first written against chapters 23–27 and had to
be renumbered to 28–32 mid-flight: a sibling agent's Urdu oblique-case tranche
landed on main and took exactly those chapter numbers, path ids
(`UR-PATH-024`–`028`), extension ids (`UR-EXT-034`–`038`) and sequence range.
**Every one of those namespaces is allocated by taking the current maximum and
adding one, so two branches off the same base collide on all of them at once**,
and git reports it as forty-five add/add conflicts rather than as the single
fact it is. The repair is to rebuild from the new base rather than to resolve
the conflicts — the generator is parameterised, so renumbering is a substitution
and a re-run, but only because the tranche was generated rather than hand-typed.

One shape worth reusing: the ordinals payoff was the LAST lesson in the track,
which made its own performance atom never-revisited by construction. A closing
retrieval lesson after the payoff fixes it, and every tranche whose chapter is
the last in its track has the same hazard.

**STILL OPEN.** `russian` holds exactly one numeral, *odin*, taught as half of a
joining pattern; `chinese` teaches *yi*–*wu* and nothing from *liu* to *shi*;
`french` and `german` reach twenty against a point demanding a hundred and want
a different repair.
