## Unreleased — Chapters 31–37: 35 more words on pre-A1 nodes

Sanskrit's pre-A1 vocabulary criterion moves 69/300 → 104/300. The shortfall
falls 231 → 196, a drop of exactly 35 — one per lesson, which is the proof that
every lesson landed on a pre-A1 spine node rather than merely on a good word.

Seven chapters appended after chapter 30, chained from `SA-C30-anjali`, one
pre-A1 node per chapter and all seven nodes reused a second time. Node reuse is
the mechanism here, not a workaround: there are only seven pre-A1 nodes in the
whole spine and the vocabulary criterion, not node coverage, is what binds.

- **Ch. 31 — The Sky and Its Lights** (`SPINE-MEET-GREET`): आकाशः, चन्द्रः,
  तारा, वायुः, हिमम्. Closes on the snow word so the reader can take
  *Himālaya* apart for themselves.
- **Ch. 32 — The Tree and What Grows** (`SPINE-EXCHANGE-NAMES`): तरुः, पत्रम्,
  तृणम्, लता, दारु. The tree lesson deliberately says nothing about दारु,
  because explaining the shared root there would make chapter 32's first page
  point forward to its last.
- **Ch. 33 — The River and the Road** (`SPINE-TAKE-LEAVE`): नदी, सेतुः,
  पर्वतः, पुरम्, समुद्रः — with पुरम् beside the Greek *polis* behind *police*.
- **Ch. 34 — More of the Body** (`SPINE-CHECK-WELLBEING`): बाहुः, ग्रीवा,
  अस्थि, रक्तम्, देहः, taking the body count to ten across two chapters.
- **Ch. 35 — In the House** (`SPINE-POLITE-REQUEST-REPAIR`): दुग्धम्, मधु,
  लवणम्, पुस्तकम्, शय्या. पुस्तकम् is flagged as the one borrowed word of the
  chapter, in from Persian *post* and back out to six modern languages.
- **Ch. 36 — More Short Replies** (`SPINE-RESPOND-BASIC`): अपि, सर्वम्, तथा,
  मन्दम्, सम्यक्. मन्दम् is a repair word, not a filler: it is what you say
  when somebody is talking faster than you can follow.
- **Ch. 37 — More Courtesy** (`SPINE-COURTESY-THANK`): शान्तिः, सादरम्,
  प्रसन्नः, अभिनन्दनम्, आशीर्वादः, closing on the *-vāda* that आशीर्वादः
  shares with धन्यवादः from the first page.

One new headword per lesson (HL14); reuse of already-taught words is unlimited
and is what makes the ramp gentler — R1 improves 0.2922 → 0.2895 with its
numerator held at 1106 while the denominator grows.

Every candidate was grepped against all 161 existing lessons with the
forward-reference detector's own word-boundary regex, and separately checked for
sitting **inside** an existing multi-word headword. Two words were caught only
by that second check and by nothing else: *atha*, which sits inside
`SA-C03-katham`'s romanization, and *aśvaḥ*, which sits inside
`SA-C28-day-after`'s *paraśvaḥ*. Neither appears in any lesson body. A third,
प्रीतिः, was dropped because `SA-C09-snihyati` already explains प्रिय at
length; teaching it later would have made chapter 9 point forward.

Every headword is spelled entirely from the 35 Devanagari glyphs the track's own
writing lessons teach, so the tranche adds **zero** script-closure violations
(sanskrit holds at 56) and zero exposure-exempted glyphs. `forwardReferences`
holds at its 500 ceiling and `ruleStatements` at 30. Book builds clean with
XeLaTeX: exit 0, 281 pages, 0 missing characters, 0 overfull, 0 underfull.

