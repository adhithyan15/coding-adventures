## Unreleased — Chapters 45–51: 35 more words on pre-A1 nodes

Sanskrit's pre-A1 vocabulary criterion moves 139/300 → 174/300. The shortfall
falls 161 → 126, a drop of exactly 35 — one per lesson, which is the proof that
every lesson landed on a pre-A1 spine node rather than merely on a good word.

Seven chapters appended after chapter 44, chained from `SA-C44-blue`, one pre-A1
node per chapter and all seven nodes reused a FOURTH time. There are only seven
pre-A1 nodes in the whole spine and the vocabulary criterion, not node coverage,
is what binds, so a fourth pass over the same seven is the shape the gate asks
for.

- **Ch. 45 — Times of the Day** (`SPINE-MEET-GREET`): प्रातः, मध्याह्नः,
  सायम्, मासः, संवत्सरः. मासः is named against चन्द्रः: the reader has had the
  moon overhead for a long time and now gets the moon counted.
- **Ch. 46 — Sun and Shade** (`SPINE-CHECK-WELLBEING`): रविः, रश्मिः,
  प्रकाशः, छाया, तापः. प्रकाशः and छाया both cost one headword and pay back an
  older one — प्रकाशः is आकाशः's own shining piece with a different prefix,
  and छाया is what छत्रम् makes.
- **Ch. 47 — Parts of a House** (`SPINE-POLITE-REQUEST-REPAIR`): भित्तिः,
  वातायनम्, सोपानम्, स्तम्भः, तलम्. भित्तिः is पुरम् brought down to the size
  of a room: the walled town was already the reader's, the wall itself was not.
- **Ch. 48 — More Animals** (`SPINE-EXCHANGE-NAMES`): सिंहः, वृकः, छागः,
  वानरः, हंसः. वानरः is assembled out of two words the reader already owns,
  वनम् and नरः, so it costs one headword and practises two older ones.
- **Ch. 49 — Five Small Answers** (`SPINE-RESPOND-BASIC`): तर्हि, यथा,
  केवलम्, मुहुः, अहो. यथा is nearly free: तथा was taught earlier and the two
  are one correlative pair, so the second half arrives already half-known.
- **Ch. 50 — Sound and Song** (`SPINE-COURTESY-THANK`): शब्दः, प्रश्नः,
  वाणी, गीतम्, श्लोकः. प्रश्नः is पृच्छति held still as a thing instead of
  running as an action, and the pair is said together on the page.
- **Ch. 51 — More of the Family** (`SPINE-TAKE-LEAVE`): पतिः, पत्नी,
  पितामहः, मातामही, मातुलः. पितामहः and मातामही are both built out of words
  already taught — पिता, माता and महत् — so the joining is watched once and
  then read a second time unaided.

One new headword per lesson (HL14); reuse of already-taught words is unlimited
and is what makes the ramp gentler — R1 improves 0.2584 → 0.2565 with its
numerator held at 1181 while the denominator grows 4570 → 4605.

**The mechanized class-(d) check ran FIRST, and it reshaped the word list rather
than validating it** (backlog HL-C214). Every Devanagari token in the sanskrit
tree was extracted (495 distinct) and the set of lesson headwords subtracted
(263), leaving **267 tokens the reader has been handed a meaning for without
ever being taught**. Sanskrit is the track most exposed to this, exactly as the
backlog predicted: it explains roots constantly, and its fourteen writing
lessons share one stock phrase. Reading the report before choosing words caught
three candidates that no substring, headword or romanization scan reaches:

- मृगः "deer" — `SA-C16-road` glosses it in full (*mṛgaḥ*, "deer": the animal
  that gets tracked) and even asks the reader to say it back. Dropped; वृकः
  took the slot.
- कथा "a story" — `SA-C03-katham` glosses *kathā* as "a telling, a story —
  the how of a thing". It appears there in romanization only, so no Devanagari
  token scan could ever have reached it. Dropped; श्लोकः took the slot.
- शीतम् "cold" — `SA-C31-snow` says हिमम् "is snow, and cold generally", so
  the gloss was already spent. Dropped; रश्मिः took the slot.

Three more were caught by the same check and KEPT, because reading the
sentences showed the collision was the lesson rather than a duplicate:

- सिंहः "a lion" — the reader has been handed it TWICE inside names, in
  `SA-C33-town`'s *Siṁhapura*, "the lion town" behind *Singapore*, and
  `SA-C20-man`'s *Narasiṁha*, "man-lion". The lesson now says so: this is a
  word you have already met twice, here it is on its own.
- शब्दः "a sound" — three earlier headwords END in it (*kāla-śabdāḥ*,
  *ka-śabdāḥ*, *saṅkhyā-śabdāḥ*) and none of them ever names it. That is the
  same shape as round three's शीलम्, except that this time the answer is to
  teach the piece rather than to drop the word, because the piece is genuinely
  missing from the reader's hands.
- प्रकाशः "light" — `SA-C31-sky` glosses आकाशः as built on "a piece meaning
  to shine, to be visible", so the root had been given away. The lesson draws
  the contrast the earlier one left undrawn: one root, two prefixes.

छाया and भित्तिः were examined under the same check and kept for the same
reason: `SA-C41-parasol` says "the shade held over a *rājā*" and `SA-C33-town`
glosses पुरम् "a walled town", but neither sentence glosses a Sanskrit word —
they describe one in English. Both new lessons now name the thing the older
sentence only gestured at.

Two candidates were dropped for consistency with earlier rounds rather than for
anything new: **अश्वः**, dropped in round two and again in round three for
sitting inside `SA-C28-day-after`'s *paraśvaḥ*, and **अथ**, dropped in round two
for sitting inside `SA-C03-katham`'s romanization. Both surfaced again on this
round's shortlist and both were dropped a third time; छागः and तर्हि took the
slots. **हस्ती** "an elephant" was dropped on the same rule — *asti* sits inside
*hastī* — even though its real relation, to हस्तः, would have made good teaching.
पात्रम् and तन्तुः, dropped in round three, were not reconsidered.

Every headword is spelled entirely from the 35 Devanagari glyphs the track's own
writing lessons teach, and so is every Devanagari citation in the bodies —
कुटुम्बम् was romanized instead, because ट is untaught. The tranche therefore
adds **zero** script-closure violations (sanskrit holds at 56, the corpus at
756), **zero** exposure-exempted glyphs (holds at 262) and **zero** new shown
glyphs (holds at 53). `forwardReferences` holds at 502, `ruleStatements` at 30,
`lessonsWithFindings` at 121, and sanskrit's cross-chapter prose references at
20 — no chapter number is named anywhere in the new prose. `metalanguage`'s
worst terms are unmoved, so no lesson reached for *verb* or *noun*.

The whole sanskrit tree was swept for per-word script purity (HL-C202/C203/
C208): roughly 3.1 million characters across 440 UTF-8 files, classified by BLOCK RANGE
rather than by `unicodedata.name` (which raises on unassigned codepoints) and
with ASCII apostrophe and hyphen treated as word boundaries. **Zero mixed-script
words and zero ZWJ/ZWNJ.** The detector was self-tested against eight synthetic
fixtures built from `chr()` — including an unassigned U+0378 wedged between
Devanagari and Latin, and a macron-and-dot-below romanization that must NOT
report — and then run against `git show` copies of the three files this repo
records as defective, rediscovering the Malayalam/Tamil mixture in
`ML-C11-nirangal.md` and `malayalam/roadmap.md` and the Latin-plus-Tamil word in
`TE-C37-noru.md` before its clean result on sanskrit was believed. (The Telugu
hit is not quoted here: pasting it would put a mixed-script word into the
sanskrit tree and make this file the next thing the sweep reports.)

Verified: vitest baseline measured on a clean tree first at 45 files / 805
tests, and 45 / 805 after; all 22 books build with XeLaTeX at exit 0 with
`book.log` and `book.pdf` both present and 0 missing characters, 0 overfull and
0 underfull boxes each — sanskrit 329 → 378 pages; `check:books`,
`check:narration`, `check:modality` and `check:progress` all exit 0.

Eight test pins moved, each by hand and each an integer plus an appended
comment: `chapter-modality-book` +7 chapters, `continuity`'s R2 +35, and
`modality-manifest`'s `totalLessons`, `voice`, `drivableLessons` and
`drivablePrefixTotal` +35 with `chapterCount` and `fullyDrivableChapters` +7.
Every move is exactly +35 or +7 and every one has the same mechanism: 35 new
ear-only lessons in 7 new chapters, each chapter drivable end to end. The
track-count assertion `toHaveLength(22)` four lines above the changed line in
`chapter-modality-book.test.ts` is untouched.

