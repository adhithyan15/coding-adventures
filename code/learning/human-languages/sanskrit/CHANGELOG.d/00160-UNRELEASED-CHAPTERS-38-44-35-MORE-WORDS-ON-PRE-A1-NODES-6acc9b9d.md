## Unreleased — Chapters 38–44: 35 more words on pre-A1 nodes

Sanskrit's pre-A1 vocabulary criterion moves 104/300 → 139/300. The shortfall
falls 196 → 161, a drop of exactly 35 — one per lesson, which is the proof that
every lesson landed on a pre-A1 spine node rather than merely on a good word.

Seven chapters appended after chapter 37, chained from `SA-C37-blessing`, one
pre-A1 node per chapter and all seven nodes reused a THIRD time. There are only
seven pre-A1 nodes in the whole spine and the vocabulary criterion, not node
coverage, is what binds, so a third pass over the same seven is the shape the
gate asks for.

- **Ch. 38 — Fire and Stone** (`SPINE-MEET-GREET`): अग्निः, भस्म, शिला,
  सिकता, मृत्तिका. अग्निः is named against दहति, the burning already taught:
  the reader has had the action for a long time and now gets the thing.
- **Ch. 39 — Animals** (`SPINE-CHECK-WELLBEING`): धेनुः, विहगः, मत्स्यः,
  शुनकः, सर्पः. The track had no animal at all before this chapter.
- **Ch. 40 — In the Kitchen** (`SPINE-POLITE-REQUEST-REPAIR`): दधि, शर्करा,
  नवनीतम्, शाकम्, मोदकः. नवनीतम् is assembled out of two words the reader
  already owns, *nava* and the *nī* inside नयति, so it costs one headword and
  practises two older ones.
- **Ch. 41 — Things People Make** (`SPINE-EXCHANGE-NAMES`): चक्रम्, कलशः,
  प्लवः, दर्पणः, छत्रम्.
- **Ch. 42 — Five More Replies** (`SPINE-RESPOND-BASIC`): कदाचित्, सहसा,
  प्रायः, किञ्चित्, बहु. कदाचित् and किञ्चित् are the same *cit* twice, on
  कदा and on किम्, so the second one is nearly free.
- **Ch. 43 — Good Manners** (`SPINE-COURTESY-THANK`): आतिथ्यम्, विनयः,
  नम्रता, विश्वासः, सरलः. आतिथ्यम् is अतिथिः stretched from the guest to
  what is owed the guest, and it names the sequence the reader can already
  say: आसनम्, पानीयम्, अन्नम्.
- **Ch. 44 — Colours** (`SPINE-TAKE-LEAVE`): श्वेतः, श्यामः, पीतः, हरितः,
  नीलः. रक्तः, the obvious red, was left out: रक्तम् is already the blood
  word and the two are the same stem.

One new headword per lesson (HL14); reuse of already-taught words is unlimited
and is what makes the ramp gentler — R1 improves 0.2723 → 0.2700 with its
numerator held at 1127 while the denominator grows 4139 → 4174.

Collision checking ran in four passes, because the track now carries 196
lessons and two rounds of everyday vocabulary. Plain prose and headword-substring
scans dropped the obvious repeats (नेत्रम् and वदनम् against अक्षि and मुखम्;
दिनम्, रात्रिः, सदा; पुनः, सत्यम्, एवम्, अस्तु, स्वस्ति, शुभम्, कृपया,
प्रणामः, प्रसन्नः). Two were caught ONLY inside an existing headword: आदरः,
which sits inside `SA-C37-respectfully`'s सादरम्, and अश्वः, which sits inside
`SA-C28-day-after`'s *paraśvaḥ* — the same substring the last round recorded and
cleared; it was dropped this time rather than cleared, and सर्पः took the slot.
पात्रम् was dropped for its romanization alone, being one macron away from
`SA-C32-leaf`'s *patram*.

SIX were caught only by READING the etymology notes, where the word never
appears as a token at all (backlog HL-C207):

- अवश्यम् "certainly" — `SA-C01-am-na` already glosses *bāḍham* as "certainly".
- चिरम् "for a long time" — `SA-C20-old-person` explains पुरातन as "it has
  existed a long time".
- ग्रन्थिः "a knot" — `SA-C33-mountain` glosses *parvan* as "a knot, a joint".
- करुणा "compassion" — `SA-C29-please` glosses *kṛpā* as "compassion, kindness".
- शीलम् "character" — all fourteen writing lessons gloss their headword as
  "the single character X", so the gloss was already spoken for.
- मृत्तिका kept its headword but changed its gloss from "earth" to "soil",
  because `SA-C08-pathati` glosses पृथिवी as "the earth, the wide one".

तन्तुः "a thread" was dropped as adjacent rather than identical:
`SA-C30-garland` opens with "Threaded, the flowers become a माला".

Every headword is spelled entirely from the 35 Devanagari glyphs the track's own
writing lessons teach, and so is every Devanagari citation in the bodies — राजा
and पुष्पम् were romanized instead, because ज and ष are untaught. The tranche
therefore adds **zero** script-closure violations (sanskrit holds at 56) and
**zero** exposure-exempted glyphs (holds at 262). `forwardReferences` holds at
its 511 ceiling, `ruleStatements` at 30, and sanskrit's cross-chapter prose
references at 20 — no chapter number is named anywhere in the new prose.

`SA-C37-blessing`, the previous last lesson, was read before anything was
appended (HL-C201) and needed no rewording: its "before the last word of the
chapter" is about chapter 37 and stays true, and it makes no claim about the
book ending.

A whole-tree per-word script-purity sweep found zero mixed-script words, zero
Latin runs carrying an Indic combining mark, and zero NUL/ZWJ/ZWNJ/BOM/bidi/
soft-hyphen characters across all 377 files. The detector was self-tested
against synthetic dirty and clean fixtures AND required to rediscover the
backlog's documented defects in `git show HEAD:` copies of
`telugu/lessons/TE-C37-noru.md` and `telugu/chapters.json` before its clean
result on sanskrit was believed.

Book builds clean with XeLaTeX: exit 0, 329 pages, 0 missing characters,
0 overfull, 0 underfull.

