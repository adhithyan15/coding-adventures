- **#14986: fifteen bones, twelve of which their own citation never mentions.**
  `biology/skeleton-bones.adj` warranted every row with the FEMUR row's sentence, on a MedlinePlus
  **leg image page**. Measured against that page, **12 of the 15 bones do not appear on it at all** —
  so a recall of the occipital bone came back proved by a sentence about the thigh. What is different
  here from the earlier entries is not the count — `food-groups` has 23 unmentioned rows and
  `word-families` 31 — but that the envelope's **page** could not support twelve of these rows under
  any quotation, not merely its chosen sentence. (That comparison is only about tables whose pages I
  can read; `food-groups`' citations are currently unreadable, per #15139.)

  The rows were never ungrounded. Their evidence sat in the file's header, listing a source, locator
  and span per bone group — where no engine could reach it. Each row now carries its own, across
  **seven pages**: five MedlinePlus, one StatPearls chapter, and the SEER divisions page for the
  framing envelope.

  ### The skull rows are re-grounded, not moved

  The header's skull quote was:

  > "An infant's skull is made up of 6 separate cranial (skull) bones: Frontal bone, Occipital bone,
  > Two parietal bones, Two temporal bones."

  **That string occurs zero times on the page it named.** There the four bones are a bulleted list,
  and the commas joining them to the lead-in sentence were invented — the same defect class as the
  invented `=` in `language/contraction.adj` (#15151), found the same day. SEER's axial-skeleton page
  lists them too. StatPearls *"Anatomy, Head and Neck, Skull"* states them in prose, so those four
  rows now cite it, widened to include the sentence naming the skull (its own opens *"It is composed
  of…"*).

  So this table needed both halves: eleven rows moved to evidence that already existed, and four
  re-grounded on a source that states rather than lists.

  ### Pins

  **13 of 13 mutants killed, two controls counted separately.** Pointing a skull row back at the leg
  page; un-widening the skull span to its anaphoric sentence; **restoring the comma-welded list quote
  this entry removed**; rebinding a region; dropping a row's locator so it inherits the framing page;
  deleting a row outright; and **every shared sentence broken in each direction separately** — tibia
  and fibula, radius and ulna, clavicle and sternum, parietal.

  ### The first version of both the tests and the mutants was drawn around its own gap

  Review found it: `assert_bone` was called for **one bone per shared span** — femur, tibia, humerus,
  scapula, ribs, patella — so `fibula`, `radius`, `ulna`, `clavicle` and `sternum` were pinned by
  nothing. `clavicle`'s source could be replaced with *"Made up entirely."* and the suite stayed
  green. Nine mutants survived a sweep.

  The mutation set did not catch this **because I had written it around the same blind spot**: it
  broke the tibia copy (covered) and not the fibula copy (not covered), the scapula row and not the
  clavicle row, ribs and not sternum. "7 of 7 killed" was true, and measured the half I had tested.
  A harness built by the same hand that wrote the tests inherits its gaps; that is what an
  adversarial reviewer is for.

  Now every one of the 15 rows is bound by BONE, never by region — `arm` and `leg` have three rows
  each, and a multi-answer query lets a needle be satisfied by a sibling's intact copy, the defect
  mutation found in `joint-types` below.

  One assertion was also **incapable of failing**: `!out.contains("\"R\":\"skull\",\"B\"")`, under a
  comment claiming it guarded the central defect. Bindings serialize `B` before `R`, so that needle
  could never appear whatever the data said. Replaced with a count — the leg page must warrant
  exactly the three leg rows (six occurrences, since provenance is emitted under `citations` and
  again under `steps`).

  Controls: unmutated green, and a fabricated envelope green — disclosed, because once every row
  overrides `source` and `locator` the envelope's wording is unreachable from any answer.

  `patella → knee` is disclosed as a reading rather than a quote-match: the page says *"Your kneecap
  is called the patella"*, and `knee` is the region that names.

  The skull rows now disclose a step that was easier to miss than that one: their span says the
  **calvaria**, *"the uppermost part of the skull"*, is composed of those four bones, so
  `frontal → skull` is part-of-a-part-of. Sound and auditable, but not a bare quote match — and it
  was going to ship undisclosed while the weaker `patella` reading was spelled out twice.

