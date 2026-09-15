- **#14070 installment 3b: thirteen glyph-flattened citations repaired across `language/*`, with
  twenty header quote lines synced across fifteen files.** Eleven single-quote repairs — three
  contraction (`figurative-language-type`, `possessive-noun`, `sentence-type`) and eight paired
  (`phoneme-addition`, `phoneme-deletion`, `phoneme-segmentation`, `phoneme-substitution`,
  `plural-s-sound`, `syllable-blending`, `syllable-count`, `syllable-substitution`) — plus two
  double-quote repairs (`silent-e-word`, `syllable-type-alias`).

  Every value now verifies verbatim against its page; the old forms appear on none of them.

  **Paired cases are not contraction cases.** The page quotes a word as `‘bike’` — U+2018 opening,
  U+2019 closing. Applying the contraction rule (every `'` becomes U+2019) yields `’bike’`, which
  appears on no page at all: it would swap one wrong citation for a differently wrong one.

  Three mutation classes pass: restore-the-flattened-form reddens all thirteen; **curl both ends the
  same way** — the mistake anyone would actually make — reddens all eight paired ones; flatten-the-
  double-quotes reddens both new ones.

  **This installment's real subject is that the screens kept being wrong, and each one was wrong in a
  way the previous fix had already taught me.** Five successive screens, five findings, every one
  raised by review rather than by me:

  1. The header sync matched a whole span against a single `%` line — so five headers that wrap the
     sentence across lines shipped still showing the flattened form of *the exact sentence the file
     now cites*. This is verbatim the 3a defect that the entry below describes. Writing the lesson
     down did not prevent repeating it one installment later.
  2. Matching atomic units instead fixed the wrap but overshot: it curled apostrophes in the header's
     own **prose** (`figurative-language-type`'s "something that isn't human" is my sentence, not the
     page's). Page-verification caught it before it shipped. A repair that cannot be verified against
     the page is not a repair.
  3. A joined-header token probe missed `onset-rime` because column annotations break contiguity.
  4. Extracting `"..."` spans from the joined header failed because **quote pairing is unreliable** —
     column cells sit between quotes, so the regex paired the close of one with the open of the next
     and extracted `map m ap` instead of the sentence.
  5. Sliding windows of each value against the joined header still passed `onset-rime`, because that
     file carries **both** copies — the correct curly form at line 68 and the flattened one at line
     34 — so "is this value present in the header?" answered yes and hid the bad copy. The comparison
     had to be made per-occurrence.

  The common shape: each screen encoded what I already believed the defect looked like. The screen
  that finally works extracts nothing and pairs nothing — it slides windows, compares under a
  normalization **blind to which quote character is used**, and checks raw bytes at each match site.
  It has a known floor: a header quoting a fragment shorter than the window is invisible to it
  (`possessive-noun:7`'s 16-character "the dog's collar" was found by review, not by it), and
  lowering the window to catch such fragments produces false positives on prose and on coincidental
  overlaps between two different rules.

  **An entire quote class had never been screened.** Every glyph screen in this effort keyed on an
  ASCII *apostrophe*. `silent-e-word` and `syllable-type-alias` ship a sentence whose only quotes are
  escaped ASCII **doubles**, where the page renders curly ones — so they contained nothing any screen
  looked for, and were invisible by construction rather than by accident. The same sentence was
  flattened in both libraries. A repo-wide double-quote screen now exists; it found one more,
  `biology/consumer-trophic-level:128`, which ships with the non-`language/*` batch.

  **`plural-s-sound`'s pin deliberately queries `dogs`, not `hats`.** That table ships one envelope,
  "In all other cases we pronounce ‘s’ as /z/", which grounds only the `z_sound` row. A pin on `hats`
  would pair an /s/ answer with a citation stating the /z/ rule and freeze it in a test, where it
  reads as an endorsement. Filed as #14124.

  Pins are anchored on the bindings through the `trust` field. Queries come from each library's
  authored `.query.adj` — **except `plural-s-sound`**, for the reason above. A derived
  `table(firstRow, $X)` assumed 2-arity and abstained on six of eleven; `phoneme_segmentation` has
  four columns. The test file for `silent-e-word` is `facts_silentEword_e2e.rs`, **with a capital
  E** — the naming rule used everywhere else yields `silenteword` and matches nothing. Third filename
  trap this effort. Filenames are looked up, never derived.

  **`language/*` is NOT clean, and an earlier draft of this entry claimed it was.** The
  header/data screens report zero across `language/*`, and that is all they report: they compare a
  header against what the same file *ships*. They say nothing about whether the shipped value matches
  its page. Review found five more `language/*` libraries — `syllable-segmentation:80`,
  `syllable-deletion:78`, `diphthong-sound:108`, `silent-letter-sound:100`,
  `other-vowel-team-sound:185` — shipping flattened glyphs, seventeen sites in all.

  **They are not repaired here, and curling them would not fix them.** Each is a *composed* span: it
  differs from its page by more than glyphs, so the whole-value check returns "not found" rather than
  "flattened", which is exactly why this effort's collector never surfaced them. Repairing the quote
  marks would make the fragments match while the sentence as a whole still did not — a defect wearing
  the appearance of a fix. They need re-grounding on what the page actually says (#13934), not
  curling. Filed to #14111 with that distinction stated.

  This matters beyond bookkeeping: `syllable-segmentation:80` and the just-repaired
  `syllable-count:72` quote near-identical sentences from the *same* Reading Rockets page, so the
  repo now ships two different renderings of one source sentence until that batch lands.

  Outside `language/*` the header screen reports six sites in five files (`anatomy/brain-parts`,
  `anatomy/tooth-parts`, `biology/animal-habitat`, `biology/consumer-trophic-level`,
  `biology/neuron-parts:52`), also #14111. Header quotes of *different* sentences remain in the
  repaired files too — `possessive-noun` has four by the quoted-span screen — likewise #14111; this
  installment never verified them.

  Confirmed glyph repairs now stand at **23 repaired-or-queued plus at least 17 newly-found
  `language/*` sites** — 4 in 3a, 13 here, 6 queued elsewhere (four biology/physics,
  `biology/vitamin-deficiency-symptom:77`, `biology/consumer-trophic-level:128`), and the composed-span
  group above. **Treat every one of these totals as a lower bound.** Each has risen every time
  somebody looked with a differently-shaped instrument, and this entry has now been corrected on that
  point four times.

  533 test binaries / 1635 tests green; clippy `-D warnings` clean; `adj_stdlib_report
  --fail-on-unreferenced-tests` and `adj_stdlib_manifest --validate-json-schema` both exit 0; all
  thirteen `.query.adj` companions parse, run and abstain correctly.

