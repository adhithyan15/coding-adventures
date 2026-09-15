- `language/sound-device-type.adj` (new) -- a new `sound_device_type(device, description)`
  table names two sound devices and what each actually does
  (onomatopoeia->is_when_a_word_imitates_the_natural_sound_of_a_thing,
  alliteration->repeating_consonant_sounds_right_next_to_each_other), quoted verbatim from
  Grammarly's "20 Types of Figures of Speech: Definitions and Examples" article -- `trust
  consensus`, the same tier `figurative-language-type.adj` (a sibling library in this
  directory, sourced from a DIFFERENT Grammarly article) already uses. Picked using the
  mandatory full-tree-grep-before-scoping discipline -- `grep -rilE
  "sound_device_type|onomatopoeia|alliteration" code/specs/data/adj-facts-stdlib/` found ZERO
  hits, confirming a completely fresh topic before this file was written. WebFetch-verified
  before writing (twice -- the second pass pulled the full surrounding paragraph for each
  candidate sentence, plus the complete list of all twenty device headings on the page, to
  confirm each stands alone grammatically and to check whether any other device on the page
  also had an equally clean single defining sentence -- only these two did; the article's
  other eighteen devices each lean on a following example, a comparison to a neighboring
  device, or multiple clauses rather than one clean single-fact sentence). Only two rows are
  shipped, an intentionally smaller table than most siblings in this directory, since only
  two of the page's twenty devices carry a genuinely standalone defining sentence -- the
  honest-abstention discipline applies to table SIZE as much as to individual queries; no
  padding with weaker rows. Honest abstention on "simile" (a real figure of speech the same
  page also covers, but already grounded as its own separately-shipped library in this
  directory, `simile-meaning.adj`, so tabling it again here under a different predicate would
  duplicate coverage rather than add to it). New manifest objective
  `adj.literacy.k2.sound_device_type` (band K-2, `recall` competency, `ccss.ela` coverage
  root). New e2e test `facts_sounddevicetype_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled sound device).
