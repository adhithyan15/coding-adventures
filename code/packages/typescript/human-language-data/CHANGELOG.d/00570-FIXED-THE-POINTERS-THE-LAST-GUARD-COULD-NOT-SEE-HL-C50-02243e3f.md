### Fixed — the pointers the last guard could not see (HL-C50)

The previous change closed cross-volume *lesson ids* and left a gap it named out
loud: pointers phrased in prose, which no pattern then in place could match. This
closes it — **72 pointer sites across 59 authored files** (53 lesson sources and 6
handwritten chapters).

- **Second-person memory claims aimed at another language**: *"you learned in
  Hindi"*, *"you met in Tamil"*, *"You may remember from Latin's colour lesson"*,
  *"You've met a genuine dog-word mystery in Spanish"*. A reader holding one volume
  has met none of it.
- **Pointers at another volume's material**: *"the Spanish track"*, *"the Tamil
  book"*, *"German's lesson on you"*, *"the Hindi lesson on ऋतु"*, *"Telugu earlier
  in this arc"*, *"every other language in this arc"*.
- Every one keeps the cross-language **fact** and resolves it to the **language and
  the word** — *"the same worldview inside Tamil's pōy varugiṟēṉ"*, not *"the
  worldview you met in Tamil"*. Nothing was cut.
- **Future-tense pointers count too** — *"You'll meet these twelve names again in
  Spanish"*, *"the words you'll meet in Kannada"* — and so does *"earlier in this
  arc"* with no memory verb in front of it — including with an adjective wedged in
  (*"elsewhere in this **entire** arc"*, verbatim the phrasing fixed in Kannada while
  its Malayalam sibling was left standing). Plural material nouns too: *"the Spanish,
  Italian, French, and Portuguese **tracks**"*. All were missed by earlier sweeps of
  this same change.
- **Five of these live in handwritten chapters**, not generated ones — `bengali`,
  `kannada`, `telugu` ch01, `hindi` ch03, `latin` ch01 carry no `GENERATED FILE`
  header, so the `.tex` *is* the source there. Checked before editing, both ways.

