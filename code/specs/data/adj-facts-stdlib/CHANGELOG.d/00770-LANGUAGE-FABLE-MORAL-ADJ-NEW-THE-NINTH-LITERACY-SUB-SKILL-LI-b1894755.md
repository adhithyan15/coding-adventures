- `language/fable-moral.adj` (new) -- the NINTH literacy sub-skill library, and the FIRST to
  ground a whole-TEXT comprehension artifact rather than a word-level phonics/spelling fact: a
  classic fable's own narrator-stated moral. A new `fable_moral(fable, moral)` table names three
  fables and their own stated lessons (tortoise_and_the_hare -> "Slow but steady wins the
  race.", shepherds_boy_and_the_wolf -> "There is no believing a liar, even when he speaks the
  truth.", boy_and_the_filberts -> "Do not attempt too much at once."), quoted verbatim from
  George Fyler Townsend's classic English translation of Aesop's Fables, hosted by Project
  Gutenberg -- a legitimate public-domain literary primary source. `trust authoritative`.
  RESEARCH DISCIPLINE: of SIX candidate fables originally surveyed on the same page, only these
  THREE give a clean, unambiguous, narrator-voice closing moral, verified by reading the raw
  page text directly. The other three -- "The Ants and the Grasshopper", "The Fox and the Crow",
  and "The Lion and the Mouse" -- were deliberately EXCLUDED after verification found their
  closing line is a character's spoken dialogue (the ants' taunt, the fox's gloat, the mouse's
  own words), not the narrator's own stated moral; asserting those as "the fable's moral" the
  same way the three shipped rows are stated would overclaim what the source actually does.
  GRAMMAR DISCOVERY: the ADJ query grammar accepts a quoted-string literal as a `table` row
  VALUE, but NOT as a query argument -- a query can only ground an atom/number or bind a $Var,
  so "which fable has moral X" is answered by enumerating with `? fable_moral($F, $Moral)` and
  reading off the match, not by querying with the moral string itself as a ground argument (a
  new finding for this stdlib, documented in the file's own header for future sentence-valued
  tables). Honest abstention on "the_fox_and_the_crow". New manifest objective
  `adj.literacy.k2.fable_moral` (band K-2, `recall` competency). New e2e test
  `facts_fablemoral_e2e.rs` (3 tests: direct recall, reverse binding of all three fables, honest
  abstention on a fable whose closing line is dialogue).
