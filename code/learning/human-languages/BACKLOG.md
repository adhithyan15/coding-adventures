# Human Languages — backlog

Note on provenance: this file was EMPTY in git for its whole history until now.
Findings from the pre-A1 tranche work were recorded in commit messages and PR
bodies, which are durable and searchable, but a reader opening this file found
nothing. The entries below are the ones that change how the work is done.

## HL-C10P — Malayalam ഴ closes the leading base-consonant gap

After Malayalam chillu **ൻ** landed, Malayalam **ഴ** led the measured queue at
**11 affected realizations**. Unicode identifies U+0D34 as MALAYALAM LETTER
LLLA, but the generated consonant table omitted that script-specific extension,
so the curriculum had neither its ISO-15919 **ḻa** row nor verified formation.

Sriveenkat's 47-frame Wikimedia Commons animation writes **ഴ** in one
uninterrupted run: the left entry arch reaches the lower junction, turns
clockwise around the right loop, and descends through its inner return into the
lower hook. The new Noto-fitted path and complete 13-syllable vowel row remove
all 11 gaps while preserving the syllable matrix. Tamil independent **உ** now
leads at **11 affected realizations**.

## HL-C10Q — the last two merge-conflict surfaces are sharded

HL21 removed the conflict points in the curriculum's **data** and left the two
files every author touches regardless of what they are working on. Measured over
the last 200 human-languages commits on main: this `BACKLOG.md` was touched by
**100** of them and `human-language-data/CHANGELOG.md` by **75**, while the 23
per-language `<track>/CHANGELOG.md` files were touched **4–11 times each**. The
per-language changelogs are already partitioned by track and were left alone;
sharding a file that does not conflict buys nothing.

The corroborating experiment was PR #12690 itself. It went `DIRTY` three separate
times while two concurrent Spanish tranches were in flight, and every time the
conflicting file was **only** `CHANGELOG.md` — not one of its ~4,000 shard files
ever conflicted. A PR whose entire purpose was to remove a conflict point was
blocked three times by a conflict point it did not remove.

**Two things this migration had to get right that HL21's did not.**

First, these documents are **newest-first**. HL21's ledgers append, so a new
element takes the next ordinal and nobody renames anything. A backlog prepends,
so under ascending filename order the newest entry needs the *smallest* ordinal
and every author reaches into a shrinking gap that two of them would both grab.
The fix is that the ordinal is a **recency rank** — topmost section, highest
number — and the join walks it downward. Prepending is an append again.

Second, prose has no ids. HL21 derives a shard filename from an authored `id`
field; 106 of this file's 107 sections start `## HL-…` and one does not (*Three
rules this work keeps re-deriving*), so an id-based scheme would have had to
special-case it. The filename is derived from the heading text and the *result*
is validated instead — and the identity is an 8-hex digest of the heading, not
the ASCII-folded slug, because `source-verified Tamil ர` and `source-verified
Tamil த` fold to the same slug.

**The migration introduces no normalization.** The sharder partitions bytes at
heading boundaries and rebuilds by concatenation, so both regenerated monoliths
are byte-identical to the pre-migration files.

**Found while measuring, not fixed here:** `human-language-data/CHANGELOG.md`
carries **three** separate "Unreleased" sections — `## Unreleased` at line 3, a
second `## Unreleased` at line 952, and `## [Unreleased]` at line 1021. Two of
the three are almost certainly the residue of exactly the bad merges this work
exists to prevent: someone resolved a conflict by keeping both sides, heading
included. They survive untouched because byte-exactness outranks tidiness;
consolidating them is a separate deliberate commit for a quiet moment, and it has
to decide what those sections were meant to be released *as*.

**Also worth knowing:** sharding does not reduce CI cost for either file. Both
sit inside directories CI treats as opaque path prefixes, so a changelog-only
edit still triggers the 23-book XeLaTeX compile and a full `npm ci` +
`vitest --coverage`. The cost sharding removes is human serialization, not CI
minutes. Narrowing those path triggers is separate work.

## HL-C10O — Malayalam chillu ൻ closes the leading final-n gap

After Kannada independent **ಅ** landed, Malayalam chillu **ൻ** led the measured
queue at **11 affected realizations**. The curriculum uses this atomic final
consonant throughout first-person and masculine forms, but the canonical script
inventory still had no independently sourced row for it.

Sriveenkat's 67-frame Wikimedia Commons animation writes **ൻ** in two pen-down
runs: the left arch descends into its central stem, then one lifted right-side
run carries the upper shoulder through the outer loop, inner return, and
finishing hook above the line. The new Noto-fitted record removes all 11 gaps
without treating chillu N as base **ന** plus an unwritten vowel killer.
Malayalam **ഴ** now leads at **11 affected realizations**.

## HL-C10M — Restore the Latin writing-activity directive gate

Refreshing from `origin/main` after Malayalam chillu **ൽ** exposed two new
validation errors from the merged Latin pre-A1 writing tranche: the delayed-copy
and dictation `hl-activity` directives appeared after learner-facing prose even
though the parser contract requires them immediately after the block's
first-line knowledge metadata and before learner copy. Once reachable, both
contracts also duplicated their canonical `answer` in `accepted`, which the
activity compiler correctly rejects as an ambiguous normalized response.

Move only those two directives above the displayed instructions and remove the
redundant accepted-answer copies. This preserves their intended responses and
lesson order while restoring the shared-curriculum validation gate; no learner
prose changes.

## HL-C10N — Kannada ಅ opens the script's handwriting inventory

After Malayalam chillu **ൽ** landed, the measured queue put Kannada independent
short **ಅ** first at **11 affected realizations**. Kannada's canonical vowel row
already identified the glyph and sound, but it had no source-backed formation
order and the script still had no authored letter path at all.

Gopala Krishna A's 35-frame Wikimedia Commons animation keeps the pencil down
for one uninterrupted run: clockwise left loop, broad lower bowl,
counterclockwise right loop, then the horizontal bar returning left. The new
font-fitted path preserves those four movements on Noto Sans Kannada, removes
all 11 gaps, and leaves Malayalam chillu **ൻ** next at 11 affected
realizations.

## HL-C10L — Malayalam chillu ൽ closes the leading final-consonant gap

After Tamil **ழ** landed, the measured queue put Malayalam chillu **ൽ** first
at **12 affected realizations**. The curriculum already teaches it in words
such as **കാൽ**, **വിരൽ**, **വാതിൽ**, and **ജനൽ**, but the canonical Malayalam
inventory had no independently sourced row for that vowel-free consonant.

Sriveenkat's 97-frame Wikimedia Commons animation writes **ൽ** in one
uninterrupted run: left entry arch, clockwise central loop, upper shoulder,
clockwise right loop, and the finishing chillu hook above the line. UT Austin's
*The Malayalam Script* independently identifies **ൽ** as the chillu shared by
**ത/ല** and says the chillu marker extends above the line. The new canonical
row removes all 12 gaps without treating the final hook as a detached mark.
Kannada independent **ಅ** now leads at **11 affected realizations**.

## HL-C10K — Tamil ழ closes the highest remaining realization gap

After Persian and Urdu **خ** landed, the measured queue put Tamil **ழ** first
at **13 affected realizations**. The canonical Tamil inventory did not yet own
the letter even though the curriculum ledger and lessons already used it.

Sankaran Radhakrishnan's UT Austin *Tamil Script Learners Manual*, Appendix I,
Frame 7, numbers **ழ** as six movements in three pen-down runs: joined
movements 1–3 form the left body and bar, joined movements 4–5 form the inner
upright and broad right bowl, and movement 6 is the detached lower hook. The
bundled Noto Sans Tamil face simplifies the source's looped left body and high
bar into a retraced upright with a low crossbar, so the authored geometry fits
that printed outline while preserving the source's three-run grouping. The new
canonical row removes all 13 gaps. Malayalam chillu **ൽ** now leads at **12**.

## HL-C10J — Persian and Urdu خ close shared glyph debt separately

After Malayalam **അ** landed, the measured queue displayed shared glyph **خ**
first at **13 affected realizations**: nine in Persian and four in Urdu. The
existing Arabic row could not supply either language's provenance.

UT Austin's Persian Online alphabet demonstrates isolated **خ** at
00:49–00:54 as a body-first head and deep bowl followed by one lifted dot
above. Northwestern's *Zer o Zabar* independently identifies Urdu **خ** as the
**ج** shape with its dot above; its independent handwriting animation likewise
completes the pointed head and bowl before placing that dot. Separate canonical
rows and script-scoped ductus keys remove all 13 gaps without collapsing the
three languages' sources. The reranked queue now moves to Tamil **ழ** at 13.

## HL-C10H — Persian پ separates its sourced row from Urdu debt

After Tamil independent short **எ** landed, the measured queue displayed shared
glyph **پ** first at **14 affected realizations**. The Persian
`perso-arabic` inventory owned four of them; the other ten belong to Urdu and
must not borrow Persian provenance.

UT Austin's Persian Online freehand alphabet demonstrates isolated **پ** at
00:16–00:21: the same shallow bowl as **ب** is swept right-to-left, then three
separate dots are placed below in left, right, and lower-center order. The new
Persian-scoped record and font-fitted ductus remove those four Persian
realizations. Urdu still owns ten **پ** gaps, but the reranked queue moves first
to Malayalam independent **അ** at **13**.

## HL-C10I — Malayalam അ closes the next initial-vowel gap

After Persian **پ** landed, the measured queue put Malayalam independent short
**അ** first at **13 affected realizations**. UT Austin's *The Malayalam Script*
identifies the initial-vowel inventory as the word-initial forms and supplies a
separate click-to-play handwriting clip for **അ**.

The clip completes the left-and-central body in one joined run: outer arch,
upper turn, lower loop, central crown, and upright. After one lift, the right
outer arch descends and curls directly into its lower inner loop. The new
canonical record and font-fitted ductus remove all 13 affected realizations.
The data README's old claim that Malayalam and Telugu had zero authored letters
was also stale; it now names their verified independent-vowel rows. The
reranked queue displays shared glyph **خ** first at **13**: nine Persian
realizations and four Urdu realizations. Audit and source those script-owned
rows separately rather than borrowing Arabic provenance.

## HL-C10G — Tamil எ closes the next independent-vowel gap

After Malayalam **എ** landed, the measured queue put Tamil independent short
**எ** first at **15 affected realizations**. The UT Austin *Tamil Script
Learners Manual* Appendix I shows **எ** on the second row of Frame 5, distinct
from the first row's dental **ந** and from dependent short-e sign **ெ** in
Frame 6.

Frame 5 numbers six connected body movements from the left climb through the
top bar, inner descent, spiral, and lower foot. After one lift, movement 7 draws
the separate right upright upward. The new canonical record and font-fitted
ductus remove all 15 affected realizations. The reranked queue moves to Persian
**پ** at 14.

## HL-C10F — Malayalam എ opens the next independent-vowel row

After Urdu **و** landed, the measured queue put Malayalam independent short
**എ** first at **15 affected realizations**. UT Austin's *The Malayalam Script*
identifies initial vowels as word-initial forms, supplies a click-to-play
handwriting clip for **എ**, and notes that the lower elements of initial **e**
extend below the line.

The clip shows the compact left hook, middle bar, upright, and inner loop in one
joined run. After one lift, a second run sweeps over the broad outer arch and
ends below the line. The new canonical record and font-fitted ductus remove all
15 affected realizations. The reranked queue moves to Tamil independent short
**எ** at 15.

## HL-C10E — Urdu و closes the next shared-glyph gap

After Tamil short-e landed, the measured queue displayed shared **و** first at
**16 affected realizations**. Arabic and Persian already had independently
verified records; every uncovered realization belonged to the Urdu
`urdu-nastaliq` inventory. Northwestern's *Zer o Zabar* supplies a separate
Urdu handwriting animation that forms the looped head and continues through
the down-left tail in one pen-down run, while its prose identifies wāw as a
nonconnector with consonant and vowel readings.

The new Urdu-scoped record and ductus entry remove all **16 affected
realizations** without borrowing either neighbouring script's citation. The
reranked queue moves to Malayalam independent short **എ** at 15.

## HL-C10D — Tamil ெ closes the short-e composition gap

After Persian **ر** landed, the measured queue put Tamil short-e sign **ெ**
first at **17 affected realizations**. UT Austin's Module 6 identifies ெ as the
secondary symbol for short **e** and explicitly says it is always placed before
the primary letter.

The new mark record removes all **17 affected realizations** for **ெ** while
preserving the source's sign-before-carrier handwriting order. Because the
module does not supply a standalone directional path or lift count, the record
does not invent either. The reranked queue moves to shared glyph **و** at 16;
audit its actual script ownership before adding another record.

## HL-C10C — Persian ر keeps shared-glyph provenance separate

After Telugu independent short **అ** landed, the measured queue displayed Arabic
**ر** first at **18 affected realizations**, but Arabic and Urdu already had
their own verified records. The uncovered realizations all belonged to the
Persian `perso-arabic` inventory. UT Austin's Persian Online freehand alphabet
demonstrates independent **ر** at 01:10–01:12 as one continuous motion: descend
from the upper tip through the short stroke, then sweep left through the lower
curve.

The new Persian-scoped inventory and ductus entry removes all **18 affected
realizations** without borrowing either neighbouring script's citation. The
reranked cross-script queue now puts Tamil short-e sign **ெ** first at **17
affected realizations**.

## HL-C10B — Telugu అ opens the independent-vowel handwriting tranche

The measured queue put Telugu independent short **అ** first at **20 affected
realizations**. Sathish Shanmugam's *Write Telugu Alphabets* tracing screen
shows four numbered directional movements and two pen-down starts: movements
1–2 stay joined around the left lobe and broad lower bowl, then movements 3–4
stay joined around the right lobe and return left along the inner bar.

The canonical independent-vowel row now carries that cited two-run order and
Language Ladder fits it to the bundled Noto Sans Telugu outline. Independent
vowels remain recognition-only placeholders until their own source-backed
ductus is present, so adding **అ** removes exactly those **20 affected
realizations** rather than silently clearing the whole generated vowel table.
The reranked cross-script queue now puts Arabic **ر** first at **18 affected
realizations**.

## HL-C10A — Tamil ந is repaired against its actual Frame 5 row

The visual source audit recorded in HL-C09Z was correct: dental **ந** had been
misattributed to Frame 12's **ள** row. UT Austin's Module 5 identifies ந as the
voiced dental nasal, and Appendix I Frame 5's first row on page 193 numbers its
six movements in three pen-down runs: joined movements 1–2, joined movements
3–4, and joined movements 5–6.

The repair replaces the unsupported Frame 12 adaptation in both the canonical
letter record and Language Ladder's font-fitted ductus. No measured realization
count changes because ந already had a record; the measured queue therefore
resumes with Telugu independent vowel **అ** at **20 affected realizations**.

## HL-C09Z — Tamil ள closes its measured gap and exposes a dental-na source collision

After Kannada anusvara landed, the measured queue put Tamil retroflex lateral
**ள** first at **20 affected realizations**. UT Austin's Module 12 identifies
the letter and directs learners to Appendix I; Frame 12 on page 195 numbers six
movements in three pen-down runs, with lifts after movements 3 and 5.

The new letter record removes all **20 affected realizations** for **ள** and
preserves that sourced three-run order. The reranked measured queue moves to
Telugu independent vowel **అ** at 20.

The same visual audit found that dental **ந** currently cites this **ள** frame
and adapts it into an unsupported path. Repair **ந** against its actual
hand-movement row before continuing the measured queue; do not let the shared
Frame 12 citation conceal the provenance collision.

## HL-C09Y — Kannada anusvara closes the last 20-plus cross-script gap

After Tamil long-e landed, the measured queue put Kannada anusvara **ಂ** first
at **24 affected realizations**. Unicode's Indic-script specification identifies
U+0C82 as Kannada's consonant-nasalization sign.

The new mark record removes all **24 affected realizations** for **ಂ** and
models the encoded carrier-first composition. Because the source does not
establish a universal handwriting path or lift count, the record adds no
standalone ductus claim. The reranked queue moves to Tamil retroflex lateral
**ள** at 20.

## HL-C09X — Tamil ே closes the left-side long-e gap

After the Tamil த provenance repair landed, the measured queue put Tamil
vowel sign **ே** first at **29 affected realizations**. UT Austin's Module 7
identifies ே as the secondary symbol for long ē and explicitly says to write
that symbol before the primary consonant, even though it is pronounced after
the consonant.

The new mark record removes all **29 affected realizations** for **ே** while
preserving the source's sign-before-carrier handwriting order. Because the
module does not supply a standalone directional path or lift count, the record
does not invent either. The reranked queue moves to Kannada anusvara **ಂ** at
24.

## HL-C09W — Tamil த provenance repair restores the four-run order

The source cross-check for ட exposed that the existing dental **த** record
points to Frame 1, but UT Austin's Module 1 explicitly teaches retroflex ட,
not dental த. Appendix I places த in the final Frame 3 row continued onto page
192. Its seven numbered directions resolve into four visible pen-down runs:
movements 1–2 for the upper frame, 3–4 for the broad right bowl, 5–6 for the
compact left loop, and separate movement 7 for the low leftward tail.

The repaired record now cites the correct frame and page, reverses the first
upright to the source's upward direction, and replaces the unsupported
continuous path with three verified lifts. The measured uncovered-glyph queue
is unchanged, so Tamil vowel sign **ே** remains next at 29.

## HL-C09V — Tamil ட closes the Frame 1 gap

After Tamil **ச** landed, the measured queue put Tamil retroflex **ட** first at
**30 affected realizations**. UT Austin's Module 1 explicitly identifies ட,
and Appendix I Frame 1 numbers its left descent and rightward foot as two
joined movements.

The new inventory and ductus entry remove all **30 affected realizations** for
**ட**. The measured uncovered-glyph queue moves to Tamil vowel sign **ே** at
29, behind the newly recorded த provenance audit.

## HL-C09U — Tamil ச closes the next Frame 3 consonant gap

After the Persian and Urdu dāl records landed, the measured queue put Tamil
**ச** first at **31 affected realizations**. The UT Austin *Tamil Script
Learners Manual* presents ச in Frame 3 as three joined movements for the upper
frame and projecting middle bar, followed by one lifted movement that turns
around and closes the lower-left bowl. The vendored Noto Sans Tamil outline
preserves that same visible ச/க family relationship.

The new inventory and ductus entry remove all **31 affected realizations** for
**ச**. The reranked queue keeps Tamil first with **ட** at 30.

## HL-C09T — Persian and Urdu د keep one glyph's provenance separate

After Tamil **ய** landed, the measured queue put **د** first at **33 affected
realizations**: 22 in Persian and 11 in Urdu. Arabic already had its own
source-verified د, so treating the gap as Arabic would have hidden the actual
script ownership. Persian Online demonstrates the Persian independent form at
01:04–01:06; Northwestern's *Zer o Zabar* supplies a separate Urdu handwriting
animation and explains the independent fold, baseline, and Naskh/Nastaliq
difference.

The two new inventory and ductus entries remove all **33 affected
realizations** while retaining separate Persian and Urdu citations. The
reranked queue moves to Tamil **ச** at 31.

## HL-C09S — Tamil ய closes the largest remaining Tamil consonant gap

After Malayalam anusvara landed, the measured queue put Tamil **ய** first at
**38 affected realizations**. The UT Austin *Tamil Script Learners Manual*
introduces ய beside ப in Frame 1, and Appendix I numbers its six movements as a
single connected route: down and around the left hook, up and back down the
central upright, across the bottom, and up the right upright.

The new inventory and ductus entry remove all **38 affected realizations** for
**ய**. The reranked queue moves to Arabic **د** at 33.

## HL-C09R — Malayalam anusvara returns the queue to Tamil

After Tamil light ra landed, the measured queue put Malayalam anusvara **ം**
first at **44 affected realizations**. Unicode 17 §12.9.3 identifies U+0D02 as
MALAYALAM SIGN ANUSVARA, shows it after independent vowels and dependent vowel
signs, and requires renderers to handle it on Malayalam letters and other
supported bases.

The Malayalam inventory now models that base-first encoded composition without
inventing a universal handwriting direction or pen-lift count. This removes all
**44 affected realizations** for **ം**. The reranked queue returns to Tamil with
**ய** at 38.

## HL-C09Q — Tamil ர hands the queue to Malayalam

After Tamil dental ta landed, the measured queue put Tamil light **ர** first at
**47 affected realizations**. The UT Austin *Tamil Script Learners Manual*
introduces ர in Frame 3 as the three-movement ஈ frame plus a slightly angular
short fourth movement. Appendix I fixes those movements as three pen-down runs:
left upright, top bar, then the joined central upright and angular tail.

The new inventory and ductus entry remove all **47 affected realizations** for
**ர**. The reranked queue moves to Malayalam anusvara **ം** at 44.

## HL-C09P — Tamil த keeps the letter-ledger queue moving

After Tamil pa landed, the measured queue put Tamil dental **த** first at **50
affected realizations**. This original tranche correctly closed the letter gap,
but incorrectly attributed த to Frame 1 and modeled it as one continuous path.
HL-C09W supersedes that provenance claim with Appendix I's actual final Frame 3
row and its four-run, seven-movement order.

The new inventory and ductus entry remove all **50 affected realizations** for
**த**. The reranked queue keeps Tamil first with **ர** at 47.

## HL-C09O — Tamil ப opens the letter-ledger closure queue

After Telugu anusvara landed, the measured queue put Tamil **ப** first at **61
affected realizations**. The UT Austin *Tamil Script Learners Manual* introduces
ப in Frame 1, states the usual left-to-right and top-to-bottom hand movement,
and presents the letter directly for copying beside its close relatives ம and
ய. The fitted path descends the left upright, crosses the bottom, and rises up
the right upright as one continuous run.

The new inventory and ductus entry remove all **61 affected realizations** for
**ப**. The reranked queue keeps Tamil first with **த** at 50.

## HL-C09N — Telugu ం removes the largest remaining nasal-sign gap

After Tamil u landed, the measured cross-script queue put Telugu anusvara
**ం** first at **68 affected realizations**. Unicode's *Indic Scripts in
Unicode* identifies U+0C02 as the Telugu consonant-nasalization sign, names it
**sunna**, and distinguishes it from U+0C01 vowel-nasalizing arasunna.

The Telugu inventory now models that carrier-first composition while making no
unsourced handwriting-direction or pen-lift claim. This removes all **68
affected realizations** for **ం**. The reranked queue puts Tamil **ப** next at
61.

## HL-C09M — Tamil ு closes the largest remaining dependent-vowel gap

After Kannada halant landed, the measured cross-script queue put Tamil vowel
sign u **ு** first at **80 affected realizations**. Unicode 17 §12.6.3 names
U+0BC1 as TAMIL VOWEL SIGN U and shows that it follows a consonant in encoded
order, normally ligating with that carrier; Table 12-28 gives **க + ு → கு**
as the first concrete example.

The Tamil inventory now models that carrier-first composition while making no
unsourced handwriting-direction or pen-lift claim. This removes all **80
affected realizations** for **ு**. The reranked queue puts Telugu anusvara
**ం** next at 68.

## HL-C09L — Kannada ್ clears the final 80-plus virama gap

After Telugu virama landed, the measured cross-script queue put Kannada halant
**್** first at **87 affected realizations**. Unicode 17 §12.8.2 identifies
U+0CCD as Kannada's halant, virama, or vowel-omission sign; defines the dead
consonant as carrier plus halant; and records that its rendered form replaces
the consonant's horn. Section 12.8.3 connects those dead consonants to later
consonants in conjunct formation.

The Kannada inventory now models that carrier-first composition while making
no unsourced handwriting-direction or pen-lift claim. This removes all **87
affected realizations** for **್**. The reranked queue puts Tamil vowel sign
**ு** next at 80, followed by Telugu anusvara **ం** at 68.

## HL-C09K — Telugu ్ removes the largest remaining script gap

After Malayalam candrakkala landed, the measured cross-script queue put Telugu
virama **్** first at **104 affected realizations**. Unicode 17 §12.7.1 shows
U+0C4D suppressing the inherent vowel in **క్**, replacing the carrier's
v-shaped headstroke, and joining encoded consonant clusters whose second member
is commonly subscripted.

The Telugu inventory now models that carrier-first composition while making no
unsourced handwriting-direction or pen-lift claim. This removes all **104
affected realizations** for **్**. The reranked queue puts Kannada virama **್**
next at 87, followed by Tamil vowel sign **ு** at 80.

## HL-C09J — Malayalam ് starts the cross-script closure queue

The fresh post-Devanagari corpus report found seven incomplete script rows and
ranked individual missing glyphs by affected realizations. Malayalam
candrakkala **്** led at **133**, ahead of Telugu virama **్** at 104, Kannada
virama **್** at 87, and Tamil vowel sign **ு** at 80.

Unicode 17 §12.9.3 identifies U+0D4D as Malayalam's candrakkala, shows it in
carrier-plus-virama conjunct sequences, and documents both visible meanings:
suppression of the preceding vowel and the neutral half-u. The inventory now
models that composition without inventing a handwriting direction or pen-lift
count that the source does not specify. This removes all **133 affected
realizations** for **്**. The reranked cross-script queue starts with Telugu
virama **్** at 104 next.

## HL-C09I9 — Devanagari ळ closes the measured shared inventory

Hela Nomad's published three-array stroke data supplies the order for **ळ**:
draw both loops as one continuous figure-eight body, descend the short stem
above the right loop, then finish with the left-to-right shirorekhā. The
learner path preserves the source's center crossings while fitting the bundled
Noto Sans Devanagari outline.

This consonant removes the final **2 affected realizations** and the final
missing glyph from the shared Hindi, Marathi, and Sanskrit closure gate. The
Devanagari row is now marked complete, and the focused validator reports zero
uncovered-glyph warnings. With this measured tranche closed, the next loop
priority is a fresh cross-script gap report rather than another presumed
Devanagari letter. The largest production `script-data` batch containing
Devanagari remains below the 250 kB authored-data target at **197.33 kB**.

## HL-C09I8 — Devanagari ढ keeps its bowl and inner loop joined

Opiaterein's 22-frame teaching animation supplies the two-run order for
**ढ**: descend the right stem and continue through the broad outer bowl and
closed inner loop without lifting, then finish with the left-to-right
shirorekhā. The learner path preserves the animation's continuous body despite
its brief within-run pauses and fits the bundled Noto Sans Devanagari outline.

This consonant reduces shared Hindi and Sanskrit closure debt from **4 to 2
affected realizations** and from **2 to 1 missing glyph**. The reranked corpus
leaves only **ळ** at 2 affected realizations; source and font-fit **ळ** next.
The largest production `script-data` batch containing Devanagari remains below
the 250 kB authored-data target at **195.57 kB**.

## HL-C09I7 — Devanagari ञ separates its rising shoulder

Opiaterein's 21-frame teaching animation supplies the four-run order for
**ञ**: draw the clockwise open-left bowl, add the rightward shoulder and rise
to the headline, descend the short stem below the bowl, then finish with the
left-to-right shirorekhā. The learner path preserves the separate shoulder and
lower-stem restarts while fitting the bundled Noto Sans Devanagari outline.

This consonant reduces shared Hindi, Marathi, and Sanskrit closure debt from
**7 to 4 affected realizations** and from **3 to 2 missing glyphs**. The
reranked corpus leaves **ढ** and **ळ** tied at 2 affected realizations each;
source and font-fit **ढ** next.
The largest production `script-data` batch containing Devanagari remains below
the 250 kB authored-data target at **194.01 kB**.

## HL-C09I6 — Devanagari झ keeps its bowls and tail joined

Opiaterein's 32-frame teaching animation supplies the four-run order for
**झ**: descend the short upper stem and continue through both bowls and the
diagonal tail, add the middle crossbar, descend the right stem, then finish
with the left-to-right shirorekhā. The learner path preserves the animation's
single joined body despite its brief within-run pauses and fits the bundled
Noto Sans Devanagari outline.

This consonant reduces shared Hindi, Marathi, and Sanskrit closure debt from
**11 to 7 affected realizations** and from **4 to 3 missing glyphs**. The
reranked corpus now puts **ञ** first at 3 affected realizations, followed by
**ढ** and **ळ** at 2 each; source and font-fit **ञ** next.
The largest production `script-data` batch containing Devanagari remains below
the 250 kB authored-data target at **192.22 kB**.

## HL-C09I5 — Devanagari घ keeps its curled body continuous

Opiaterein's 22-frame teaching animation supplies the three-run order for
**घ**: sweep continuously through the upper curl, middle hook, lower bowl, and
rising right side, add the short stem below the bowl, then finish with the
left-to-right shirorekhā. The learner path preserves the animation's within-run
pause at the upper curl without inventing an extra lift and fits the bundled
Noto Sans Devanagari outline.

This consonant reduces shared Hindi, Marathi, and Sanskrit closure debt from
**17 to 11 affected realizations** and from **5 to 4 missing glyphs**. The
reranked corpus now puts **झ** first at 4 affected realizations, followed by
**ञ** at 3 and **ढ** and **ळ** at 2 each; source and font-fit **झ** next.
The largest production `script-data` batch containing Devanagari remains below
the 250 kB authored-data target at **190.33 kB**.

## HL-C09I4 — Devanagari ष retraces its body-side stem

Opiaterein's 24-frame teaching animation supplies the four-run order for
**ष**: draw the U-shaped body from the left side around the lower bowl and up
the right side, retrace that right side while descending the full stem, add the
inner diagonal, then finish with the left-to-right shirorekhā. The learner path
preserves that retraced stem while fitting the bundled Noto Sans Devanagari
outline.

This consonant reduces shared Hindi, Marathi, and Sanskrit closure debt from
**27 to 17 affected realizations** and from **6 to 5 missing glyphs**. The
reranked corpus now puts **घ** first at 7 affected realizations, followed by
**झ** at 4 and **ञ** at 3; source and font-fit **घ** next.
The largest production `script-data` batch containing Devanagari remains below
the 250 kB authored-data target at **188.52 kB**.

## HL-C09I3 — Devanagari फ retraces its central stem

JackPotte's 15-frame teaching animation supplies the three-run order for
**फ**: descend the left stem, curve around the lower bowl, rise along the
central side and retrace it downward, add the open right arch, then finish with
the left-to-right shirorekhā. The learner path preserves that distinctive
retraced first run while fitting the bundled Noto Sans Devanagari outline.

This consonant reduces shared Hindi, Marathi, and Sanskrit closure debt from
**38 to 27 affected realizations** and from **7 to 6 missing glyphs**. The
reranked corpus now puts **ष** first at 10 affected realizations, followed by
**घ** at 7 and **झ** at 4; source and font-fit **ष** next.
The largest production `script-data` batch containing Devanagari remains below
the 250 kB authored-data target at **186.64 kB**.

## HL-C09I2 — Devanagari ठ separates its stem from its closed body

Opiaterein's 18-frame teaching animation supplies the three-run order for
**ठ**: descend the short central stem, restart at its lower junction and trace
the closed round body counterclockwise, then finish with the left-to-right
shirorekhā. The learner path preserves those two body-side pen-down runs while
fitting the bundled Noto Sans Devanagari outline.

This consonant reduces shared Hindi, Marathi, and Sanskrit closure debt from
**48 to 38 affected realizations** and from **8 to 7 missing glyphs**. The
reranked corpus now puts **फ** first at 11 affected realizations, followed by
**ष** at 10 and **घ** at 7; source and font-fit **फ** next.
The largest production `script-data` batch containing Devanagari remains below
the 250 kB authored-data target at **184.74 kB**.

## HL-C09I1 — Devanagari थ keeps its spiral and lower bowl joined

Opiaterein's 27-frame teaching animation supplies the three-run order for
**थ**: curl through the upper spiral, continue from its left waist around the
broad lower bowl to the right-stem junction without lifting, descend the right
stem, then finish with the left-to-right shirorekhā. The learner path preserves
that continuous spiral-and-bowl body while fitting the bundled Noto Sans
Devanagari outline.

This consonant reduces shared Hindi, Marathi, and Sanskrit closure debt from
**60 to 48 affected realizations** and from **9 to 8 missing glyphs**. The
reranked corpus now puts **ठ** and **फ** first at 11 affected realizations each,
followed by **ष** at 10 and **घ** at 7; source and font-fit **ठ** next.
The largest production `script-data` batch containing Devanagari remains below
the 250 kB authored-data target at **183.29 kB**.

## HL-C09I0 — Devanagari ड writes its whole S-shaped body continuously

Opiaterein's 20-frame teaching animation supplies the two-run order for **ड**:
descend the right stem, turn left across the shoulder, sweep through the upper
loop and broad open lower bowl without lifting, then finish with the
left-to-right shirorekhā. The learner path preserves that continuous S-shaped
body while fitting the bundled Noto Sans Devanagari outline.

This consonant reduces shared Hindi, Marathi, and Sanskrit closure debt from
**71 to 60 affected realizations** and from **10 to 9 missing glyphs**. The
reranked corpus now puts **थ** first at 12 affected realizations, followed by
**ठ** and **फ** at 11 each and **ष** at 10; source and font-fit **थ** next.
The largest production `script-data` batch containing Devanagari remains below
the 250 kB authored-data target at **181.65 kB**.

## HL-C09HZ — Devanagari छ keeps its nested loops in one run

Opiaterein's 28-frame teaching animation supplies the three-run order for
**छ**: sweep through the upper-left loop, continue around the lower bowl and
outer right side into the inner loop without lifting, descend the short upper
stem, then finish with the left-to-right shirorekhā. The learner path preserves
that continuous nested body while fitting the bundled Noto Sans Devanagari
outline, whose upper-left loop ends at a disconnected open tip.

This consonant reduces shared Hindi, Marathi, and Sanskrit closure debt from
**85 to 71 affected realizations** and from **11 to 10 missing glyphs**. The
reranked corpus now puts **ड** first at 13 affected realizations, followed by
**थ** at 12 and **ठ** and **फ** at 11 each; source and font-fit **ड** next.
The largest production `script-data` batch containing Devanagari remains below
the 250 kB authored-data target at **180.15 kB**.

## HL-C09HY — Devanagari ट is one continuous stem-and-body run

Opiaterein's 17-frame teaching animation supplies the two-run order for **ट**:
descend the central stem, turn left through the shoulder, continue
counterclockwise around the open round body without lifting, then finish with
the left-to-right shirorekhā. The learner path preserves that continuous first
run while fitting the bundled Noto Sans Devanagari outline.

This consonant reduces shared Hindi, Marathi, and Sanskrit closure debt from
**96 to 85 affected realizations** and from **12 to 11 missing glyphs**. The
reranked corpus now puts **छ** first at 15 affected realizations, followed by
**ड** at 13 and **थ** at 12; source and font-fit **छ** next.
The largest production `script-data` batch containing Devanagari remains below
the 250 kB authored-data target at **178.39 kB**.

## HL-C09HX — Nukta closes seven carrier combinations without inventing letters

Unicode 17 defines U+093C DEVANAGARI SIGN NUKTA as a true diacritic: a
subscript dot that extends the consonant inventory. The shared script data now
models the carrier-plus-mark composition directly for the seven combinations in
the current corpus — **क़, ख़, ग़, ज़, ड़, ढ़,** and **फ़** — while explicitly
distinguishing Unicode character order from a claimed universal handwriting
order.

This one combining mark reduces shared Hindi, Marathi, and Sanskrit closure
debt from **106 to 96 affected realizations** and from **13 to 12 missing
glyphs**. The reranked corpus now puts retroflex **ट** first at 18 affected
realizations, followed by **छ** at 15 and **ड** at 13; source and font-fit **ट**
next.
The largest production `script-data` batch containing Devanagari remains below
the 250 kB authored-data target at **176.99 kB**.

## HL-C09HW — Devanagari ण keeps its bowl and inner stem joined

Opiaterein's 19-frame teaching animation supplies the three-run order for
**ण**: descend the left stem, curve clockwise around the lower bowl, and rise
along the inner right stem without lifting; descend the separate outer right
stem; then finish with the left-to-right shirorekhā. The learner path preserves
that continuous first run while fitting the bundled Noto Sans Devanagari
outline.

This consonant reduces shared Hindi, Marathi, and Sanskrit closure debt from
**132 to 106 affected realizations** and from **14 to 13 missing glyphs**. The
measured corpus now ranks nukta **़** first at 27 affected realizations, followed
by **ट** at 18 and **छ** at 15; source and model the attachment behavior of
nukta next rather than assuming the remaining consonant order.
The largest production `script-data` batch containing Devanagari remains below
the 250 kB authored-data target at **175.43 kB**.

## HL-C09HV — Devanagari ज joins its bowl to the middle bar

Opiaterein's 20-frame teaching animation supplies the three-run order for
**ज**: sweep the open hook clockwise around the lower bowl and continue through
the middle bar without lifting, descend the right stem, then finish with the
left-to-right shirorekhā. The learner path preserves that continuous first run
while fitting the bundled Noto Sans Devanagari outline.

This consonant reduces shared Hindi, Marathi, and Sanskrit closure debt from
**140 to 132 affected realizations** and from **15 to 14 missing glyphs**.
Source and font-fit **ण** next; after it, rerank the measured corpus rather than
assuming the remaining consonant order. The largest production `script-data`
batch containing Devanagari remains below the 250 kB authored-data target at
**173.88 kB**.

## HL-C09HU — Devanagari ख closes the highest-reach consonant gap

Opiaterein's 28-frame teaching animation supplies the four-run order for
**ख**: one continuous descending left body through its small loop and broad
lower bowl, a separate clockwise upper-right loop, the descending right stem,
and the left-to-right shirorekhā. The authored paths preserve that order while
fitting the bundled Noto Sans Devanagari outline, whose printed upper loop has
a small contour gap that the handwriting source crosses without lifting.

This one consonant reduces shared Hindi, Marathi, and Sanskrit closure debt
from **150 to 140 affected realizations** and from **16 to 15 missing glyphs**.
Source and font-fit **ज** next, then **ण**; they are the remaining highest-reach
consonants in the measured corpus. The largest production `script-data` batch
containing Devanagari remains below the 250 kB authored-data target at **172.26
kB**.

## HL-C09HT — Candrabindu and Visarga remove the widest Devanagari gaps

The first closure-debt tranche inventories Unicode's U+0901 Devanagari Sign
Candrabindu **ँ** and U+0903 Devanagari Sign Visarga **ः**, with attachment
descriptions and current-corpus examples. These two signs alone reduce the
fail-closed measurement from **244 to 150 affected realizations** and from **18
to 16 missing glyphs** across Hindi, Marathi, and Sanskrit.

Next, source and font-fit the consonants **ख, ज,** and **ण**. They remain the
highest-reach missing letter rows; unlike combining signs, each needs verified
stroke order, lift count, and a learner path fitted to Noto Sans Devanagari. The
largest production `script-data` batch containing Devanagari remains below the
250 kB authored-data target at **170.59 kB**.

## HL-C09HS — Devanagari closure debt is measured, not waved through

Exercising the completion gate against all three Devanagari tracks finds **244
affected lesson realizations** and **18 distinct missing glyphs**: ख, ़, ँ, ठ,
फ, ज, थ, ष, ट, ढ, झ, घ, छ, ड, ण, ळ, ः, and ञ. The inventory therefore remains
fail-closed at 28 sourced rows. Integration coverage now pins both the affected
realization count and missing set so future vocabulary or inventory expansion
cannot change this debt silently.

The next implementable tranche is the five most widespread gaps: Chandrabindu
**ँ**, Visarga **ः**, and consonants **ख, ज,** and **ण**. Each needs
source-backed stroke data and font-fit paths before the completion audit can be
repeated; no flag-only shortcut is acceptable.

## HL-C09HR — Chinese closure grows with the Mandarin corpus

The completion audit proves that all Han characters in the current Mandarin
lessons close against **29 source-verified character and radical rows**. This is
an intentionally moving logographic inventory: every new lesson headword must
add its missing character, sourced stroke order, and regenerated subset-font
coverage before validation can pass.

This hard gate outranked speculative expansion work because it protects every
future Mandarin tranche immediately. Next, audit the Devanagari starter corpus;
its 28 rows are already source-backed but shared by three growing tracks, so any
completion claim must close Hindi, Marathi, and Sanskrit together. The
production Chinese-bearing `script-data` batch remains below the 250 kB
authored-data target at **170.21 kB**.

## HL-C09HQ — Hebrew closes the current pointed lesson corpus

The completion audit proves that the current Hebrew headwords close against all
**22 source-verified consonant rows**, their encoded final forms, and the **nine
niqqud** presently used by the lessons. Completion describes real-corpus
closure, not an unsupported claim that every historical or specialist
cantillation mark has been inventoried.

This audit moved ahead of uppercase Cyrillic because it could harden an existing
false completion claim using already-authored, source-backed data; Cyrillic
capitals require 33 new verified shape rows rather than a flag audit. Next,
inventory the less-common Hebrew niqqud actually needed by planned lessons, or
begin that separately sourced uppercase Cyrillic tranche. The production
Hebrew-bearing `script-data` batch remains below the 250 kB authored-data target
at **192.26 kB**.

## HL-C09HP — Cyrillic closes the lowercase Russian lesson corpus

The completion audit proves that the current Russian lesson headwords close
against all **33 lowercase Russian letters**, with every inventory row carrying
stroke-order provenance. The completion claim is deliberately scoped to the
lowercase citation forms used by the corpus; it does not pretend that uppercase
Cyrillic shapes have already been authored.

The next highest-value script item is therefore the **uppercase Cyrillic
inventory expansion**: add the 33 capital forms with their own verified display
and writing evidence, then keep the real-corpus closure green. After that,
Hebrew's 22 sourced consonant rows are the next false completion claim to audit.
The production Cyrillic-bearing `script-data` batch remains below the 250 kB
authored-data target at **170.20 kB**.

## HL-C09HO — Arabic closure reads script, not transliteration

The completion gate now filters a headword to the Unicode script owned by its
inventory and compares both the headword and inventory in canonical decomposed
form. Legacy Latin **as-salāmu** teaching text therefore no longer becomes a
list of fake missing Arabic glyphs, while precomposed **أ, إ, and آ** resolve to
Alif plus Hamza Above, Hamza Below, or the newly inventoried Maddah Above.

Arabic can now assert corpus closure without duplicating composed characters as
base letters. The next highest-value completion audit is **Cyrillic**: its 33
sourced rows already match the Russian alphabet, but the still-false completion
claim needs the same real-corpus gate before it changes. The Arabic-bearing
`script-data` batch remains below the 250 kB authored-data target at **52.23
kB**.

## HL-C09HN — Arabic لا is a ligature, not a thirty-second letter

The closing Arabic audit models obligatory **لا** with its source-backed ordinary
two-stroke order: descend from the upper right, lift, then cross from the upper
left and finish along the baseline. The editable identity remains the two Unicode
letters **ل + ا**; U+FEFB **ﻻ** is retained only as the joined Noto Naskh outline
used by the font-fit gate.

Arabic's sourced shape audit is now complete at **31 learner rows**, **29
canonical base/standalone rows**, one seated-Hamza composition family, and one
obligatory ligature. Its separate corpus-closure flag remains false: exercising
that gate reveals romanized teaching strings plus composed **أ, إ, and آ** are
still treated as missing base characters. The next highest-priority item is to
make closure validation composition-aware and distinguish transliteration from
Arabic spelling before asserting script completeness. The production
Arabic-bearing `script-data` batch remains below the 250 kB authored-data target
at **51.92 kB**.

## HL-C09HM — Arabic ى keeps Yaa's old dotless body but not its identity

The second ending-form audit adds source-backed **ى** and connected-final **ـى**.
Its one-run S-shaped body is geometrically identical to the verified **ي** body,
but it has no dots and represents word-final long **ā**, so it retains separate
identity and provenance rather than becoming a Yaa alias or a Hamza carrier.

Arabic now exposes **31 source-verified learner rows** while retaining **29
canonical base/standalone rows**. Audit obligatory **لا** next as a ligature;
do not inflate the base-letter count. The production `script-data` batch must
remain below the 250 kB authored-data target; this build measures the
Arabic-bearing batch at **50.28 kB**.

## HL-C09HL — Arabic ة is a word-final ending, not a base-letter duplicate

The ending-form audit adds **ة** with its own source-backed isolated Naskh path:
close the compact clockwise body on the baseline, then place its two upper dots.
It records only isolated **ة** and connected-final **ـة**, because the form occurs
word-finally; the existing **ه** and **ت** rows remain independent base letters.

Arabic now exposes **30 source-verified learner rows** while retaining **29
canonical base/standalone rows** (28 base letters plus Hamza). Continue the
ending audit with **ى** next, then audit obligatory **لا** separately as a
ligature. The production `script-data` batch must remain below the 250 kB
authored-data target; this build measures the Arabic-bearing batch at **48.68
kB**.

## HL-C09HK — Arabic seated Hamza composes existing carrier paths

The Hamza lesson resolves the first post-alphabet debt without adding fake base
letters. It inventories **أ, إ, ؤ, and ئ** as Hamza combined with **ا, و,** or a
dotless **ي** seat, and explicitly orders the carrier first and Hamza afterward.
The canonical data now records U+0654 Hamza Above and U+0655 Hamza Below with
Unicode-normalized examples, composition order, and source provenance.

Arabic remains **29 unique source-verified rows** because these are compositions
of existing carrier and Hamza paths, not new alphabet rows. The next ending-form
audit is **ة** first, then **ى**; audit obligatory **لا** separately as a ligature.
The production `script-data` batch must remain below the 250 kB authored-data
target; this build measures the Arabic-bearing batch at **46.96 kB**.

## HL-C09HJ — Arabic ق closes the source-verified base-letter audit

The University of Oregon's dedicated Qaf video closes the canonical Arabic
lesson-corpus base-letter audit. Independent **ق** loops around its small closed
head and continues into its deep below-baseline bowl in one run, then lifts to
place the upper-right and upper-left dots as separate strokes.

Arabic is now **29 unique source-verified rows**: all 28 base letters plus the
independently sourced Hamza row. The base-letter audit is complete, but the
script remains deliberately incomplete until seated Hamza forms and ending
variants are explicitly inventoried rather than conflated with base letters.
Audit seated Hamza carriers **أ إ ؤ ئ** next, recording whether they need distinct
learner paths or composition metadata. The production `script-data` batch must
remain below the 250 kB authored-data target; this build measures the
Arabic-bearing batch at **44.45 kB**.

## HL-C09HI — Arabic ف joins its head to the bowl before the upper dot

The University of Oregon's dedicated Faa video extends the canonical Arabic
inventory with the next lesson-corpus base letter. Independent **ف** loops
around its small closed head and flows directly left through the broad bowl in
one run, then lifts once to place the upper dot.

Arabic is now **28 unique source-verified rows** and remains deliberately
incomplete. The lesson-corpus audit still requires one base letter: **ق**.
Continue in source order with **ق** next; separately account for seated Hamza
and ending variants instead of conflating them with new base letters. The
production `script-data` batch must remain below the 250 kB authored-data target;
this build measures the Arabic-bearing batch at **42.61 kB**.

## HL-C09HH — Arabic غ completes its ع body before the upper dot

The University of Oregon's dedicated Ghayn video extends the canonical Arabic
inventory with the next lesson-corpus base letter. Independent **غ** draws the
complete **ع** body in one uninterrupted run, then lifts once to place the upper
dot. Its own video supplies that body-first order rather than treating the
shared skeleton as proof.

Arabic is now **27 unique source-verified rows** and remains deliberately
incomplete. The lesson-corpus audit still requires two base letters: **ف and
ق**. Continue in source order with **ف** next; separately account for seated
Hamza and ending variants instead of conflating them with new base letters. The
production `script-data` batch must remain below the 250 kB authored-data target;
this build measures the Arabic-bearing batch at **41.00 kB**.

## HL-C09HG — Arabic ظ places its dot before the descending upright

The University of Oregon's dedicated emphatic-DHaa video extends the canonical
Arabic inventory with the next lesson-corpus base letter. Independent **ظ**
draws the complete **ط** body first, lifts once to place its upper dot, then
lifts again to draw the tall upright top-to-bottom. Its own video supplies that
body-dot-upright order rather than treating the shared skeleton as proof.

Arabic is now **26 unique source-verified rows** and remains deliberately
incomplete. The lesson-corpus audit still requires three base letters: **غ, ف,
and ق**. Continue in source order with **غ** next; separately account for seated
Hamza and ending variants instead of conflating them with new base letters. The
production `script-data` batch must remain below the 250 kB authored-data target;
this build measures the Arabic-bearing batch at **39.48 kB**.

## HL-C09HF — Arabic ط loops before its descending upright

The University of Oregon's dedicated emphatic-Taa video extends the canonical
Arabic inventory with the next lesson-corpus base letter. Independent **ط**
closes its oval and exits left along the baseline first, then lifts once to
draw the tall upright top-to-bottom into the body's right junction. The direct
MOV and its accessible Panopto mirror agree on that two-run order.

Arabic is now **25 unique source-verified rows** and remains deliberately
incomplete. The lesson-corpus audit still requires four base letters: **ظ, غ,
ف, and ق**. Continue in source order with **ظ** next; separately account for
seated Hamza and ending variants instead of conflating them with new base
letters. The production `script-data` batch must remain below the 250 kB
authored-data target; this build measures the Arabic-bearing batch at **37.42
kB**.

## HL-C09HE — Arabic ز keeps Raa's curve and adds its dot last

The University of Oregon's dedicated Zay video extends the canonical Arabic
inventory with the next lesson-corpus base letter. Independent **ز** draws the
complete **ر** body first — down through the short stroke and left through the
lower curve in one run — then lifts once to place the single upper dot. Its own
video supplies that order rather than treating the shared skeleton as proof.

Arabic is now **24 unique source-verified rows** and remains deliberately
incomplete. The lesson-corpus audit still requires five base letters: **ط, ظ,
غ, ف, and ق**. Continue in source order with **ط** next; separately account for
seated Hamza and ending variants instead of conflating them with new base
letters. The production `script-data` batch must remain below the 250 kB
authored-data target; this build measures the Arabic-bearing batch at **35.76
kB**.

## HL-C09HD — Arabic ذ keeps Daal's body and adds its dot last

The University of Oregon's dedicated Dhaal video now extends the canonical
Arabic inventory beyond the greeting-era starter set. Independent **ذ** draws
the complete **د** body first — down-right through the shoulder and left along
the baseline in one run — then lifts once to place the single upper dot. Its
own video supplies that order rather than treating a shared skeleton as proof.

Arabic is now **23 unique source-verified rows** and remains deliberately
incomplete. The lesson-corpus audit still requires six base letters: **ز, ط, ظ,
غ, ف, and ق**. Continue in source order with **ز** next; separately account for
seated Hamza and ending variants instead of conflating them with new base
letters. The production `script-data` batch must remain below the 250 kB
authored-data target; the current build measures **34.28 kB**.

## HL-C09HC — Arabic ء closes the source-backed starter inventory

Arabic Language Learning Notes' Naskh lesson replaces Arabic's final
conventional placeholder with
an explicitly documented one-stroke variant. Independent **ء** sweeps through
its c-shaped upper head, then continues without lifting from the lower-left end
through the lower diagonal. The source also records the accepted two-stroke
alternative, so the learner path does not erase real instructional variation.

The completion audit confirms **22/22 unique Arabic starter rows are
source-verified**, but Arabic must remain incomplete: the lesson corpus still
uses base letters absent from this starter inventory. Continue in source order
with **ذ** next, then **ز, ط, ظ, غ, ف, and ق**; separately account for seated
Hamza and ending variants instead of conflating them with new base letters.
The production `script-data` batch must remain below the 250 kB authored-data
target; the current build measures **32.71 kB**.

## HL-C09HB — Arabic ن sweeps its deep bowl before placing the dot

Waraqa Institute's adjacent beginner lesson now replaces the final body-and-dot
placeholder in Arabic's canonical sequence. Independent **ن** starts at the
upper-right tip, sweeps down and around its deep below-baseline bowl, then lifts
once to place the centred upper dot last. Arabic, Persian, and Urdu retain
independent source records for the shared glyph.

Arabic is now **21/22 verified, 1 remaining**. Finish the inventory with **ء**
next, then audit Arabic for canonical uniqueness and completion before moving to
the next script. The production `script-data` batch must remain below the 250 kB
authored-data target; the current build measures **31.46 kB**.

## HL-C09HA — Arabic م keeps its closed head and descending tail in one run

The next Arabic placeholder is now source-backed by Waraqa Institute's explicit
beginner writing lesson. Independent **م** forms its small, tightly closed head
first and continues down-left through the below-baseline tail without lifting.
The Arabic-scoped path remains independently addressable from the existing
Persian and Urdu records for the same Unicode glyph.

Arabic is now **20/22 verified, 2 remaining**. Continue in source order with
**ن** next. Arabic remains incomplete until each learner-facing row has its own
source-backed, font-checked path. Its `script-data` batch must remain below the
250 kB authored-data target; the production build measures **30.38 kB**.

## HL-C09GZ — Arabic ث adds its three dots after the shared bowl

The post-Gujarati audit promoted Arabic's earliest conventional placeholder.
The University of Oregon lesson's dedicated **ث** video draws the independent
shallow bowl right-to-left before its three separate upper dots. The fitted
Noto Naskh path preserves that body-first, four-run sequence.

Arabic is now **19/22 verified, 3 remaining**. Continue in source order with
**م** next. Arabic remains incomplete until each learner-facing row has its own
source-backed, font-checked path. Its `script-data` batch measures **29.33 kB**,
below the 250 kB authored-batch target.

## HL-C09GY — Gujarati closes with one canonical row per letter

The completion audit found that restored **ન** and **પ** still had their old
conventional placeholders later in `gujarati.json`. Those duplicate rows made
the inventory appear to contain 46 letters even though the teaching-app
sequence and the font-backed ductus cover 44 unique letters. The stale rows are
removed, and a cross-script uniqueness gate now rejects this class of drift.

Gujarati is now **44/44 source-verified and complete**. Future work should treat
this track as maintenance: repair source, font-fit, rendering, or curriculum
defects when evidence finds one, rather than manufacturing a 45th letter.
The deduplicated Gujarati-bearing `script-data` batch measures **82.13 kB**,
below its 250 kB target.

## HL-C09GX — Gujarati હ flows from its upper loop around the lower bowl

The source-and-font audit replaced **હ**'s conventional placeholder. The
t30apps.com animation gives it one populated SVG path: a continuous run circles
the compact upper loop, threads through the middle turn, and continues around
the broad lower bowl into its rightward finish. Noto Sans Gujarati prints those
features as two disconnected contours, so the fitted learner median uses the
source-backed connecting bridge to preserve that zero-lift order.

Gujarati is now **44/46 verified, 2 remaining**. Return to source order with
**ન** next, the earliest remaining conventional placeholder. The Gujarati-
bearing `script-data` batch measures **82.36 kB**, below its 250 kB target.

## HL-C09GW — Gujarati સ carries its looped body into the long shoulder

The source-and-font audit replaced **સ**'s conventional placeholder. The
t30apps.com animation gives it two populated SVG paths: one run circles the
rounded upper loop, descends through the left body, and sweeps across the long
right shoulder; after one lift, the tall right spine descends into its lower
foot. The fitted Noto Sans Gujarati medians preserve that order.

Gujarati is now **43/46 verified, 3 remaining**. Continue in source order with
**હ** next, the next conventional placeholder. The Gujarati-bearing
`script-data` batch measures **80.92 kB**, below its 250 kB target.

## HL-C09GV — Gujarati શ joins its upper loop to the lower body before the spine

The source-and-font audit replaced **શ**'s conventional placeholder. The
t30apps.com animation gives it two populated SVG paths: one run circles the
small upper loop and continues through the broad lower body into its tail;
after one lift, the tall right spine descends into its lower foot. The fitted
Noto Sans Gujarati medians preserve that order.

Gujarati is now **42/46 verified, 4 remaining**. Continue in source order with
**સ** next, the next conventional placeholder. The Gujarati-bearing
`script-data` batch measures **79.30 kB**, below its 250 kB target.

## HL-C09GU — Gujarati વ completes its rounded body before the right spine

The source-and-font audit replaced **વ**'s conventional placeholder. The
t30apps.com animation gives it two populated SVG paths: one run circles the
broad rounded left body and returns into the right shoulder; after one lift,
the tall right spine descends into its lower foot. The fitted Noto Sans Gujarati
medians preserve that order.

Gujarati is now **41/46 verified, 5 remaining**. Continue in source order with
**શ** next, the next conventional placeholder. The Gujarati-bearing
`script-data` batch measures **77.74 kB**, below its 250 kB target.

## HL-C09GT — Gujarati ળ keeps its bowl, arch, and spine in one run

The source-and-font audit replaced **ળ**'s conventional placeholder. The
t30apps.com animation gives it one populated SVG path: one continuous run
circles the broad left bowl, rises through the narrow middle turn, crosses the
high right arch, and descends the tall spine into its foot. The fitted Noto Sans
Gujarati median preserves that zero-lift order.

Gujarati is now **40/46 verified, 6 remaining**. Continue in source order with
**વ** next, the next conventional placeholder. The Gujarati-bearing
`script-data` batch measures **76.18 kB** after the deterministic group split,
below its 250 kB target.

## HL-C09GS — Gujarati લ separates its rounded body, shoulder, and spine

The source-and-font audit replaced **લ**'s conventional placeholder. The
t30apps.com animation gives it three populated SVG paths: the broad rounded
left body is drawn first, the separate middle shoulder follows after one lift,
and the tall right spine descends into its foot after a second lift. The fitted
Noto Sans Gujarati medians preserve that order.

Gujarati is now **39/46 verified, 7 remaining**. Continue in source order with
**ળ** next, the next conventional placeholder. The Gujarati-bearing
`script-data` batch measures **244.77 kB**, below its 250 kB target.

## HL-C09GR — Gujarati ર keeps its upper body, loop, and tail continuous

The source-and-font audit replaced **ર**'s conventional placeholder. The
t30apps.com animation gives it one populated SVG path: one continuous run
circles the rounded upper body, narrows through the middle loop, and descends
into the lower-right tail. The fitted Noto Sans Gujarati median preserves that
zero-lift order.

Gujarati is now **38/46 verified, 8 remaining**. Continue in source order with
**લ** next, the next conventional placeholder. The Gujarati-bearing
`script-data` batch measures **242.99 kB**, below its 250 kB target.

## HL-C09GQ — Gujarati ય completes its rounded body before the right spine

The source-and-font audit replaced **ય**'s conventional placeholder. The
t30apps.com animation gives it two populated SVG paths: one run circles through
the rounded upper turn, sweeps around the broad lower body, and exits across the
long right shoulder; after one lift, the tall right spine descends into its
lower foot. The fitted Noto Sans Gujarati medians preserve that order.

Gujarati is now **37/46 verified, 9 remaining**. Continue in source order with
**ર** next, the next conventional placeholder. The Gujarati-bearing
`script-data` batch measures **241.54 kB**, below its 250 kB target.

## HL-C09GP — Gujarati મ joins its left turn to the shoulder before the spine

The source-and-font audit replaced **મ**'s conventional placeholder. The
t30apps.com animation gives it two populated SVG paths: one run curls through
the left body and compact inner turn before exiting across the long shoulder;
after one lift, the tall right spine descends into its lower foot. Gujarati is
now **36/46 verified, 10 remaining**; continue in source order with **ય** next.
The Gujarati `script-data` batch measures **239.91 kB**, below its 250 kB target.

## HL-C09GO — Gujarati ભ joins its loop and inner turn before the right spine

The source-and-font audit replaced **ભ**'s conventional placeholder. The
t30apps.com animation gives it two populated SVG paths: one continuous run
circles the broad left loop, winds through the compact inner turn, and exits
across the long right shoulder; after one lift, the separate tall right spine
descends into its lower foot. The fitted Noto Sans Gujarati medians preserve
that order.

Gujarati is now **35/46 verified, 11 remaining**. Continue in source order with
**મ** next, the next conventional placeholder. The Gujarati-bearing
`script-data` batch measures **238.35 kB**, below its 250 kB target.

## HL-C09GN — Gujarati બ completes its rounded body before the right spine

The source-and-font audit found **બ** represented only by a conventional
placeholder. The t30apps.com animation gives it two populated SVG paths: one continuous run
circles the rounded left body, winds through the compact inner turn, and exits
across the right shoulder; after one lift, the separate tall right spine
descends into its lower foot. The restored entry and fitted Noto Sans Gujarati
medians preserve that order.

Gujarati is now **34/46 verified, 12 remaining**. Continue in source order with
**ભ** next; it is the next conventional placeholder. A correctness defect
still outranks coverage if one appears during fitting or validation.

The Gujarati-bearing `script-data` batch must remain below its 250 kB authored-
batch target. The production build measures it at **236.70 kB** after this
addition. Split the batch rather than weakening learner-facing provenance if
this or a later entry crosses the target.

## HL-C09GM — Gujarati ફ completes its winding body before the cross-stroke

The source-and-font audit found **ફ** missing from `gujarati.json`. The
t30apps.com animation gives it two populated SVG paths: one continuous run
sweeps left across the high cap, winds down and around the body, circles the
small lower-left loop, and exits through the descending tail; after one lift,
a separate diagonal cross-stroke rises from lower left to upper right. The
restored entry and fitted Noto Sans Gujarati medians preserve that order.

Gujarati is now **33/46 verified, 13 remaining**. Continue in source order with
**બ** next; it is also missing from the current inventory and therefore
outranks later conventional placeholders. A correctness defect still outranks
coverage if one appears during fitting or validation.

The Gujarati-bearing `script-data` batch must remain below its 250 kB authored-
batch target. The production build measures it at **235.05 kB** after this
addition. Split the batch rather than weakening learner-facing provenance if
this or a later entry crosses the target.

## HL-C09GL — Gujarati પ joins its high left hook to the lower body

The source-and-font audit found **પ** missing from `gujarati.json`. The
t30apps.com animation gives it two populated SVG paths: one run curls through
the high left hook, descends the left stem, and sweeps around the broad lower
body into the right shoulder; after one lift, the separate tall right spine
descends into its lower foot. The restored entry and fitted Noto Sans Gujarati
medians preserve that order and the observed lift.

Gujarati is now **32/45 verified, 13 remaining**. Continue in source order with
**ફ** next; it is also missing from the current inventory and therefore
outranks later conventional placeholders. A correctness defect still outranks
coverage if one appears during fitting or validation.

The Gujarati-bearing `script-data` batch must remain below its 250 kB authored-
batch target. The production build measures it at **233.10 kB** after this
addition. Split the batch rather than weakening learner-facing provenance if
this or a later entry crosses the target.

## HL-C09GK — Gujarati ન restores its loop, shoulder, and tall spine

The source-and-font audit found **ન** missing from `gujarati.json`. The
t30apps.com animation gives it two populated SVG paths: one run circles the
small left loop and continues right across the long shoulder; after one lift,
the separate tall right spine descends into its lower foot. The restored entry
and fitted Noto Sans Gujarati medians preserve that order and the observed lift.

Gujarati is now **31/44 verified, 13 remaining**. Continue in source order with
**પ** next; it is also missing from the current inventory and therefore
outranks later conventional placeholders. A correctness defect still outranks
coverage if one appears during fitting or validation.

The Gujarati-bearing `script-data` batch must remain below its 250 kB authored-
batch target. The production build measures it at **231.24 kB** after this
addition. Split the batch rather than weakening learner-facing provenance if
this or a later entry crosses the target.

## HL-C09GJ — Gujarati ધ separates its joined body from the tall spine

The t30apps.com animation gives **ધ** two populated SVG paths: first a joined
run descends from the high left entry, curls through the upper turn, narrows
through the middle, and sweeps around the broad lower body into the right
shoulder; after one lift, the tall right spine descends into its foot. The
fitted Noto Sans Gujarati medians preserve that order and the observed lift.

Gujarati is now **30/43 verified, 13 remaining**. Continue in source order with
**ન** next; it is missing from the current inventory and therefore outranks
later conventional placeholders. A correctness defect still outranks coverage
if one appears during fitting or validation.

The Gujarati-bearing `script-data` batch is **229.51 kB** in this production
build, below its 250 kB authored-batch target. Recheck it on every source note;
split the batch rather than weakening learner-facing provenance if a later
entry crosses the target.

## HL-C09GI — Gujarati દ keeps its upper and lower turns continuous

The t30apps.com animation gives **દ** one populated SVG path. The pen circles
the rounded upper body from its upper-right start, narrows through the middle
turn without lifting, then sweeps around the broad lower body and rises into
the lower-right terminal. The fitted Noto Sans Gujarati median preserves that
continuous order while following the printed glyph's wider bowls and compact
terminal.

Gujarati is now **29/43 verified, 14 remaining**. Continue in source order with
**ધ** next; it is present in the inventory but still has only a conventional
placeholder. A correctness defect still outranks coverage if one appears
during fitting or validation.

The Gujarati-bearing `script-data` batch is **227.79 kB** in this production
build, below its 250 kB authored-batch target. Recheck it on every source note;
split the batch rather than weakening learner-facing provenance if a later
entry crosses the target.

## HL-C09GH — Gujarati થ restores its looped body and tall spine

The source-and-font audit found **થ** missing from `gujarati.json`. The
t30apps.com animation gives it two populated SVG paths: one continuous run
circles the small upper loop, descends through the middle, sweeps around the
broad lower body, and rises into the right shoulder; after one lift, a separate
tall spine descends into its lower foot. The restored entry and fitted Noto Sans
Gujarati medians preserve that order and the observed lift.

Gujarati is now **28/43 verified, 15 remaining**. Continue in source order with
**દ** next; it is present in the inventory but still has only a conventional
placeholder. A correctness defect still outranks coverage if one appears
during fitting or validation.

The Gujarati-bearing `script-data` batch is **226.28 kB** in this production
build, below its 250 kB authored-batch target. Recheck it on every source note;
split the batch rather than weakening learner-facing provenance if a later
entry crosses the target.

## HL-C09GG — Gujarati ત separates its open body from the tall spine

The t30apps.com animation gives **ત** two populated SVG paths: first the pen
starts at the lower terminal, sweeps left and upward around the open body, and
continues right across the upper shoulder; after one lift, it descends the tall
right spine and turns through the lower foot. The fitted Noto Sans Gujarati
medians preserve that order while following the printed glyph's wider body and
straighter spine.

Gujarati is now **27/42 verified, 15 remaining**. Continue in source order with
**થ** next; it is missing from the current inventory and therefore outranks
later conventional placeholders. A correctness defect still outranks coverage
if one appears during fitting or validation.

The Gujarati-bearing `script-data` batch is **224.41 kB** in this production
build, below its 250 kB authored-batch target. Recheck it on every source note;
split the batch rather than weakening learner-facing provenance if a later
entry crosses the target.

## HL-C09GF — Gujarati ણ separates its hooked body, bowl, and spine

The t30apps.com animation gives **ણ** three populated SVG paths: first the left
spine flowing into its hooked lower tail, then the separate rounded middle
bowl, and finally the tall right spine and lower foot. The fitted Noto Sans
Gujarati medians preserve that order and the two observed lifts, including the
printed body's deeper below-baseline tail.

Gujarati is now **26/42 verified, 16 remaining**. Continue in source order with
**ત** next; it is present in the current inventory but still needs cited ductus
verification. A correctness defect still outranks coverage if one appears
during fitting or validation.

The Gujarati-bearing `script-data` batch is **222.85 kB** in this production
build, below its 250 kB authored-batch target. Recheck it on every source note;
split the batch rather than weakening learner-facing provenance if a later
entry crosses the target.

## HL-C09GE — Gujarati ઢ keeps its outer bowl and inner loop continuous

The t30apps.com animation gives **ઢ** one populated SVG path: the pen begins at
the upper left, sweeps right across the high shoulder, descends through the
middle and around the broad outer lower bowl, then continues directly around
the small inner loop without lifting. The fitted Noto Sans Gujarati median
preserves that shoulder-to-bowl-to-loop order and zero-lift evidence.

Gujarati is now **25/42 verified, 17 remaining**. Continue in source order with
**ણ** next; it is present in the current inventory but still needs cited ductus
verification. A correctness defect still outranks coverage if one appears
during fitting or validation.

The Gujarati-bearing `script-data` batch is **221.24 kB** in this production
build, below its 250 kB authored-batch target. Recheck it on every source note;
split the batch rather than weakening learner-facing provenance if a later
entry crosses the target.

## HL-C09GD — Gujarati ડ repairs the adjacent retroflex inventory gap

The canonical-order audit found **ડ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`; the prior backlog note
incorrectly called it present. Its animation uses one populated SVG path: the
pen begins at the upper right, sweeps left across the high shoulder, descends
through the middle and around the broad lower bowl, then finishes at the lower
left without lifting. The restored entry and fitted median preserve that order.

Gujarati is now **24/42 verified, 18 remaining**. Continue in source order with
**ઢ** next; it is present in the current inventory but still needs cited ductus
verification. A correctness defect still outranks coverage if one appears
during fitting or validation.

The Gujarati-bearing `script-data` batch is **219.74 kB** in this production
build, below its 250 kB authored-batch target. Recheck it on every added source
note; split the batch rather than weakening learner-facing provenance if a
later entry crosses the target.

## HL-C09GC — Gujarati ઠ closes the next consonant inventory gap

The canonical-order audit found **ઠ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`. Its animation uses one
populated SVG path: the pen begins at the upper right, sweeps left across the
high shoulder, descends through the middle and around the broad outer lower
bowl, then curls back inward to its terminal without lifting. The restored
inventory entry and fitted median preserve that order and zero-lift evidence.

Gujarati is now **23/41 verified, 18 remaining**. Continue in source order with
**ડ** next; it is present in the source and current inventory but still needs
cited ductus verification. A correctness defect still outranks coverage if one
appears during fitting or validation.

The Gujarati-bearing `script-data` batch is **217.89 kB** in this production
build, below its 250 kB authored-batch target. Keep rechecking it as the cited
inventory grows; split the batch rather than weakening or deleting learner-
facing provenance if a later entry crosses the target.

## HL-C09GB — Gujarati ટ keeps its upper turn and lower bowl continuous

The t30apps.com animation gives **ટ** one populated SVG path: the pen begins at
the upper left, sweeps across the upper turn, bends diagonally down-left through
the middle, then circles the broad lower bowl and finishes on its right side
without lifting. The fitted Noto Sans Gujarati median preserves that zero-lift
order.

Gujarati is now **22/40 verified, 18 remaining**. Continue in source order with
**ઠ** next; it is present in the source and font but absent from the inventory.
A correctness defect still outranks coverage if one appears during fitting or
validation.

The Gujarati-bearing `script-data` batch is now **244.12 kB** against its
250 kB authored-batch target. Recheck the production bundle before publishing
the next source note; split that batch rather than weakening or deleting
learner-facing provenance if the next entry crosses the target.

## HL-C09GA — Gujarati ઞ restores its body, shoulder, and tall spine

The canonical-order audit found **ઞ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`. Its animation uses
three populated SVG paths: the rounded left body, a short separate rightward
shoulder, and a final tall descending spine that curls into its lower terminal.
The restored entry and fitted medians preserve that order and the two observed
lifts.

Gujarati is now **21/40 verified, 19 remaining**. Continue in source order with
**ટ** next; it already has a conventional inventory entry but still needs cited
ductus verification. A correctness defect still outranks coverage if one
appears during fitting or validation.

## HL-C09FZ — Gujarati ઝ repairs the next consonant inventory gap

The canonical-order audit found **ઝ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`. Its animation uses
three populated SVG paths: the rounded left body, a separate right loop-and-tail
run, and a final short descending upper stem. The restored entry and fitted
medians preserve that order and the two observed lifts.

Gujarati is now **20/39 verified, 19 remaining**. Continue in source order with
**ઞ** next; it is also absent from the current inventory. A correctness defect
still outranks coverage if one appears during fitting or validation.

## HL-C09FY — Gujarati જ keeps its left loop, crossing body, and right loop joined

The t30apps.com animation gives **જ** one populated SVG path: the pen circles
the upper-left loop, continues diagonally through the crossing body, circles
the lower-right loop, and sweeps into the long upper-right exit without lifting.
The fitted Noto Sans Gujarati median preserves that zero-lift order.

Gujarati is now **19/38 verified, 19 remaining**. Continue in source order with
**ઝ** next; it is absent from the current inventory. A correctness defect still
outranks coverage if one appears during fitting or validation.

## HL-C09FX — Gujarati છ keeps both rounded bodies in one continuous run

The t30apps.com animation gives **છ** one populated SVG path: the pen circles
the upper-left lobe, turns back through the middle, continues around the broad
lower body, climbs the outer right curve, and finishes by circling the
upper-right lobe without lifting. The fitted Noto Sans Gujarati median preserves
that zero-lift order.

Gujarati is now **18/38 verified, 20 remaining**. Continue in source and
inventory order with **જ** next. A correctness defect still outranks coverage
if one appears during fitting or validation.

## HL-C09FW — Gujarati ચ joins its upper bowl, middle loop, and lower body

The t30apps.com animation gives **ચ** two populated SVG paths: one continuous
run circles the upper bowl, turns through the small middle loop, and continues
around the broad lower body; after one lift, a separate run descends the full
right spine and turns through its lower foot. The fitted Noto Sans Gujarati
medians preserve that joined-body-before-spine order.

Gujarati is now **17/38 verified, 21 remaining**. Continue in source and
inventory order with **છ** next. A correctness defect still outranks coverage
if one appears during fitting or validation.

## HL-C09FV — Gujarati ઙ repairs the adjacent consonant inventory gap

The canonical-order audit found **ઙ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`. Its animation uses two
populated SVG paths: one long S-like run flows from the upper-right through the
upper turn and rounded lower body to the lower-left; after one lift, a compact
separate loop draws the upper-right dot. The restored entry and fitted medians
preserve that body-before-dot order.

Gujarati is now **16/38 verified, 22 remaining**. With the adjacent consonant
inventory gaps repaired, continue in source order with existing **ચ** next. A
correctness defect still outranks coverage if one appears during validation.

## HL-C09FU — Gujarati ઘ repairs the first consonant inventory gap

The canonical-order audit found **ઘ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`. Its animation uses two
populated SVG paths: one continuous run flows through the upper lobe, middle
turn, and rounded lower body; after one lift, a separate run descends the full
right spine and turns through its lower foot. The restored entry and fitted
medians preserve that joined-body-before-spine order.

Gujarati is now **15/37 verified, 22 remaining**. Continue in source order with
**ઙ** next; it is also absent from the current inventory. A correctness defect
still outranks coverage if one appears during fitting or validation.

## HL-C09FT — Gujarati ગ separates its rounded body from the right spine

The t30apps.com animation gives **ગ** two populated SVG paths: one continuous
run circles the rounded left body from its upper-left start to its lower-left
finish; after one lift, a separate run descends the full right spine and turns
through its lower foot. The fitted Noto Sans Gujarati medians preserve that
body-before-spine order.

Gujarati is now **14/36 verified, 22 remaining**. Continue in source and
inventory order with **ઘ** next. A correctness defect still outranks coverage
if one appears during fitting or validation.

## HL-C09FS — Gujarati ખ separates its joined body from the right spine

The t30apps.com animation gives **ખ** two populated SVG paths: one continuous
run begins at the upper left, descends through the left lobe, curls through the
middle, and finishes beside the right spine; after one lift, a separate run
descends the full right spine and turns through its lower foot. The fitted Noto
Sans Gujarati medians preserve that joined-body-before-spine order.

Gujarati is now **13/36 verified, 23 remaining**. Continue in source and
inventory order with **ગ** next. A correctness defect still outranks coverage
if one appears during fitting or validation.

## HL-C09FR — Gujarati ક begins consonant coverage; ખ is next

The t30apps.com animation gives **ક** two populated SVG paths: one continuous
run circles the upper loop, crosses diagonally into the rounded lower body, and
finishes at the lower left; after one lift, a separate diagonal sweeps from
lower left to upper right. The fitted Noto Sans Gujarati medians preserve that
joined-body-before-cross-stroke order.

Gujarati is now **12/36 verified, 24 remaining**. Continue in source and
inventory order with **ખ** next. A correctness defect still outranks coverage
if one appears during fitting or validation.

## HL-C09FQ — Gujarati ઋ closes the canonical-vowel inventory gap; ક is next

The canonical-order audit found **ઋ** in both the t30apps.com source and bundled
Noto Sans Gujarati font but absent from `gujarati.json`. Its animation uses
three populated SVG paths: a bent left body, a separately descended central
stem, and a final right loop-and-tail run. The restored entry and fitted medians
preserve that order and the two observed lifts.

Gujarati is now **11/36 verified, 25 remaining**. With the source-backed
independent-vowel gaps repaired, continue to the first existing consonant,
**ક**, next. A correctness defect still outranks coverage if one appears during
fitting or validation.

## HL-C09FP — Gujarati ઔ repairs the second adjacent vowel gap; ઋ is next

The source-and-font audit found **ઔ** missing from `gujarati.json`. Its
t30apps.com animation repeats the four verified **ઓ** runs—joined body, first
stem, trailing stem, and lower high arc—then adds a fifth, higher arc. The new
inventory entry and Noto Sans Gujarati fit preserve all five paths and four
observed lifts.

Gujarati is now **10/35 verified, 25 remaining**. Rechecking canonical vowel
order exposed another inventory gap at **ઋ**, which appears in both the same
teaching source and bundled font but not the current data. Repair **ઋ** before
starting **ક**; a correctness defect still outranks coverage if one appears
during fitting or validation.

## HL-C09FO — Gujarati ઓ combines the verified આ body with one high arc

The t30apps.com animation exposes **ઓ** as four populated SVG paths: the joined
body, two separately descended stems, and a final high arc. The fitted Noto Sans
Gujarati medians therefore reuse the verified **આ** three-run structure before
adding **એ**'s high-mark movement, preserving all three observed lifts.

Gujarati is now **9/34 verified, 25 remaining**. Continue to **ઔ** next: the
source-and-font audit found another missing independent vowel rather than a
merely unverified entry, and its adjacent animation adds a second high arc to
the **ઓ** sequence. Repairing that inventory gap outranks the later consonants;
a correctness defect still outranks coverage if one appears during validation.

## HL-C09FN — Gujarati ઐ repairs an inventory gap; ઓ is next

The source-and-font audit found that **ઐ** was missing from `gujarati.json`, not
merely unverified. The t30apps.com animation gives it the first three **એ** runs
plus a fourth, higher arc; the new entry and Noto fit preserve all four paths
and the resulting three lifts. Gujarati's inventory denominator therefore rises
instead of hiding the omission.

Gujarati is now **8/34 verified, 26 remaining**. Continue to **ઓ** next: its
adjacent animation has four populated paths—one joined body, two separately
descended stems, and one high arc—so it can reuse verified **આ** and **એ**
anatomy. A correctness defect still outranks coverage if one appears during
fitting or validation.

## HL-C09FM — Gujarati એ establishes a three-run pattern; ઐ is next

The t30apps.com animation gives **એ** three populated SVG paths: a joined left
bowl, lower body, and small right arch; a separately descended full-height
right stem; then a separate high arc. The fitted Noto Sans Gujarati medians
preserve that body-before-stem-before-arc order and both observed lifts.

Gujarati is now **7/33 verified, 26 remaining**. Continue to **ઐ** next: its
adjacent animation repeats the same first three runs and adds a fourth high arc,
providing direct evidence for three lifts. Reusing the verified body, stem, and
first arc makes that the lowest-risk next tranche. A correctness defect still
outranks coverage if one appears during fitting or validation.

## HL-C09FL — Gujarati ઊ extends ઉ without lifting; એ is next

The t30apps.com animation repeats the complete **ઉ** path for **ઊ**, then keeps
the pen down across a high shoulder and along the extended right-side tail to
its lower foot. The fitted Noto Sans Gujarati median reuses the verified bowls
and outer curve before entering that added printed tail.

Gujarati is now **6/33 verified, 27 remaining**. Continue to **એ** next: its
adjacent animation changes the source pattern to three populated SVG paths, so
the next tranche should verify the joined body, separate right-side run, and
high arcing run plus the resulting two lifts. A correctness defect still
outranks coverage if one appears during fitting or validation.

## HL-C09FK — Gujarati ઉ verifies the bowl-and-return pattern; ઊ is next

The t30apps.com animation gives **ઉ** as exactly one populated SVG path. It
circles the small upper bowl through the middle cusp, sweeps around the broad
lower bowl, then climbs the tall outer-left curve to finish at the upper right.
The fitted Noto Sans Gujarati median preserves that whole zero-lift sequence.

Gujarati is now **5/33 verified, 28 remaining**. Continue to **ઊ** next: its
adjacent animation repeats the complete **ઉ** path and then extends the same
unbroken run into a long right-side tail. That direct reuse makes it the
lowest-risk next provenance and fitting tranche. A correctness defect still
outranks coverage if one appears during fitting or validation.

## HL-C09FJ — Gujarati ઈ confirms the adjacent source pattern; ઉ is next

The next t30apps.com animation also has exactly one populated SVG path. It
repeats **ઇ** through the upper loop, middle crossing, and lower loop, then
extends the rising motion into **ઈ**'s high clockwise curl. The fitted Noto Sans
Gujarati median stays on the taller printed outline without inventing a lift.

Gujarati is now **4/33 verified, 29 remaining**. Continue to **ઉ** next: it is
the next independent vowel in source order and keeps this tranche inside one
already-audited source before moving to ઊ. A correctness defect still outranks
coverage if one appears during fitting or validation.

## HL-C09FI — Gujarati ઇ is the next verified ductus; ઈ now outranks the rest

The t30apps.com version-1.0 animation gives **ઇ** as exactly one populated SVG
path: small upper-left loop, narrow middle crossing, broad lower loop, then the
rising upper-right hook. Its other path slots are empty, so this is positive
zero-lift evidence rather than a guess based on the finished outline. The fitted
Noto Sans Gujarati median stays on the bundled glyph's ink and preserves that
upper-loop → lower-loop → hook order.

Gujarati is now **3/33 verified, 30 remaining**. The next item is **ઈ**: it is
adjacent in the same source, reuses the newly checked loop-and-hook anatomy, and
therefore has lower provenance and fitting risk than jumping to a different
script or a later Gujarati letter. After ઈ, continue in source order through
ઉ and ઊ unless a correctness defect outranks coverage.

## HL-C213 — build all 22 books LOCALLY before pushing; it takes ~100 seconds

Measured on a 14-core laptop, clean rebuild, 8-way parallel:

```
all 22 books        ~100s wall clock
sequential sum      ~367s
spanish alone         74s  (1337pp — sets the parallel floor)
```

The same work in CI takes **5 to 58 minutes**, and once hung **6 hours** in `apt`
fetching the TeX toolchain, having compiled nothing. So the LaTeX work is not the
cost — provisioning and queueing are.

`data/scripts/build_all_books.sh` runs the lot and checks the four things that
matter. **The exit code alone is not one of them:**

| check | why the exit code misses it |
|---|---|
| missing characters | a font gap prints NOTHING and still exits 0 — telugu once shipped 89 |
| overfull boxes | spanish crossed 1000pp, contents numbers gained a digit, 14 lines overflowed, exit 0 |
| underfull boxes | the fix for overfull can trade one for the other |
| exit code | catches only a hard LaTeX error |

`sh build_all_books.sh --self-test` proves each detector fires on a known-dirty log
and stays silent on a clean one. It is not decoration: **it caught two real bugs in
this script before either ever ran on the corpus** —

1. `grep -c` EXITS 1 when the count is zero, so `|| echo 0` appended a second zero
   and the integer test errored instead of comparing. That silently disabled the
   missing-character check — the very thing the script exists for.
2. The overfull/underfull probes never fired, because `printf '%s'` does not
   interpret backslashes in its ARGUMENT, so `\\hbox` stayed two backslashes.

A rebuild overwrites `book.log`, so a defect planted in a real log cannot survive
long enough to test the detector. That is why classification is factored into
`classify_log` and tested against synthetic logs instead.

## Three rules this work keeps re-deriving

**A measurement that did not happen looks exactly like one that passed.** Check the
magnitude, not just the verdict: a vitest run with a bad `--reporter` flag failed at
startup and EXITED 0; a scan with a wrong cwd read zero files and reported clean;
`grep $'\x00'` is an empty pattern that matches every file. Read the exit code, the
`Test Files N passed (N)` line, AND the count against a known baseline.

**Verify a checker against a known-dirty case before believing a clean result.**
Eleven detectors in this work reported clean while blind — from `\w` excluding the
combining marks under inspection, to an unassigned codepoint splitting a
mixed-script word into two clean halves, to a heredoc mangling a detector's own
fixtures.

**Fix the thing measured, never the threshold.** A gate on the LARGEST eager chunk
was once satisfied by splitting one 502 kB chunk into four — the page still
downloaded the same bytes. The real fix made the data lazy: 502 kB → 287 kB, and no
corpus growth reaches the eager graph at all.


## HL-C214 — class (d) is MECHANIZABLE: glossed-but-never-a-headword

HL-C207 named the collision class no string search reaches — a candidate that
collides with a GLOSS rather than a token — and said mechanising it would need a
schema change. **It does not.** The Hindi round-3 tranche derived it from data
already in the corpus:

1. extract every script token appearing anywhere in the track (Hindi: 587 distinct);
2. subtract the set of tokens that are some lesson's HEADWORD;
3. what remains — **67 tokens for Hindi** — is the set of words the reader has been
   handed a meaning for without ever being taught them.

Every class-(d) collision found by hand on five previous tracks lands in that set.
Hindi's discards from it include अलमारी and चाबी (both glossed inline in a ROOM
lesson listing Portuguese loans), ठंड (glossed as the stock phrase ठंड है), मीठा
(glossed inside another word's etymology), and तिल (glossed in an OIL lesson).
Four of those were on the shortlist and would have shipped.

**This is a report, not a gate.** The set is a candidate list to READ, not to reject
automatically: the same tranche cleared मिट्टी, बीज, अनाज and टोकरी after reading
the sentences, because the gloss was of an English word, a different word, or an
absence. A rule that only rejects is a filter.

**KNOWN LIMIT, found by running it first on sanskrit:** the check operates on
SCRIPT tokens, so a word glossed only in ROMANIZATION is invisible to it. Sanskrit's
कथा was caught by a person reading `SA-C03-katham`, which glosses *kathā* as "a
telling, a story" without ever printing the Devanagari. **Run the same three steps a
second time over romanized forms**, or accept that a human still has to read the
etymology notes. The mechanized pass narrows the field; it does not close it.

Sanskrit's numbers, running it FIRST rather than last: **267 glossed-but-never-taught
tokens out of 495 distinct**. It reshaped the word list instead of invalidating it —
3 discarded (मृगः glossed outright in a ROAD lesson; कथा; शीतम् glossed inside
"himam is snow, and cold generally"), 3 turned INTO teaching (सिंहः, handed over
twice inside the names *Siṁhapura* and *Narasiṁha*; शब्दः, which three earlier
headwords END in without any of them naming it; प्रकाशः, whose काश् root a SKY
lesson had already given away), and 2 examined and kept because the colliding
sentence was an English description rather than a gloss.

Worth building as `report --glossed-not-taught <track>`. Until then, the three
steps above are cheap to run inline and catch the class that nothing else does.

## HL-C215 — two pre-existing mixed-script typos in Hindi, reported not fixed

A per-word purity sweep of all 485 files in the Hindi tree found exactly two, both
shipping into the PDF and the narration:

```
HI-C33-shubh-dopahar.md:40   "the widened सense"   DEVANAGARI SA standing in for Latin s
HI-C32-shubh-sandhya.md:54,76 "रात्रि became राat"  should be रात
```

Both render as plausible wrong text — the same class as the Telugu and Malayalam
defects already fixed under HL-C202, and invisible to any check that looks at one
script at a time.

**Left unfixed deliberately.** They sit in chapters unrelated to the tranche that
found them, and touching them would pull those chapters' book and narration hashes
into an unrelated diff. They want their own small change.

Not a defect, do not "fix": `HI-C25-din.md:56` carries Proto-Slavic `*dьnь`, whose
Cyrillic yers inside a Latin transliteration are correct scholarly notation.

## HL-C216 — the CI "appendix warnings" are multi-pass noise; the artifact is clean

Reported from reading CI logs: lots of warnings around the appendix. **Investigated;
the books are clean. Nothing to fix, and the reason is worth writing down so the
next person does not re-open it.**

### What the CI log actually contains

The books build step emits **8,541 warning lines**. Nearly all are one of four,
once per track:

```
22x  Package rerunfilecheck Warning: File `book.out' has changed
22x  Package hyperref Warning: Rerun to get /PageLabels entry
22x  LaTeX Warning: There were undefined references
22x  LaTeX Warning: Label(s) may have changed. Rerun to get cross-references right
 9x  Package polyglossia Warning: hyphenation patterns for modern Latin
```

**These are INTERMEDIATE-PASS artifacts.** `latexmk` runs xelatex repeatedly; on the
first pass the `.aux` has no labels yet, so every cross-reference is undefined and
LaTeX asks to be re-run. By the final pass they resolve. Checked directly: the
individually-alarming `Reference 'ch:ru-script-eleven' undefined` and
`ch:fa-script-nine` both **are** defined -- one definition each, referenced 13 and 11
times -- and **do not appear undefined in any final `book.log`.**

### Why they look appendix-related

They are not. `latexmk` prints each file as it opens it, so the log interleaves

```
(./chapters/appendix-glossary.tex [97] [98] [99])
```

with warning text from the same pass. The appendix files are simply the LAST and
LONGEST things processed -- spanish's index is 4,739 lines -- so they sit next to the
end-of-pass warning burst. **Proximity in the log, not causation.**

### The measurement that matters

The repo's own scanner reads the FINAL `book.log` and counts six classes. Both
locally and in CI it reports, for all 22 tracks:

```
overfull=0 underfull=0 missing_character=0 hyperref_warning=0
duplicate_destination=0 font_substitution=0
```

The only non-zero values anywhere are spanish `font_substitution=2` (a bold-mono
face CI lacks; cosmetic, already understood) and one russian `underfull`. The
`latin: ... [over baseline]` lines in CI are the scanner's OWN self-test fixtures,
not corpus results -- worth knowing, since they look like a failing track.

### What would be a real finding

A warning that survives into the FINAL log. `data/scripts/build_all_books.sh` reads
exactly that, which is why it reports clean while the raw CI log looks alarming.
**Judge the artifact, not the transcript.**

### The one real defect it surfaced -- FIXED in this change

`ch:how-are-you` was defined twice, in spanish chapters 8 AND 9, because
`chapters.json` carried the same `label` on both. That one DID survive to the final
log, and it was not cosmetic: the index referenced that single target **eight**
times, four of them printed as *"Chapter 8, p.N"* -- but `\pageref` resolves to
whichever definition LaTeX saw last, which is chapter 9. **Every index entry
pointing at chapter 8 printed chapter 9's page number.** A reader following it lands
on the wrong page.

Chapter 9 now carries `ch:answering`. The index splits 4/4 across the two targets,
and the `multiply defined` warning -- the last real warning in the whole corpus -- is
gone. All 22 books rebuild clean.

**Worth noting how it was found:** not by the warning, which had been visible and
ignored for a long time as "pre-existing and cosmetic", but by asking what the
warning would actually DO to a reader. The warning named a duplicate label; the
defect was a wrong page number in an index.
