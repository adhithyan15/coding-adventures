- `language/noun-type.adj` (extended) -- adds three more noun types
  (concrete_noun->perceived_by_the_senses_physical_or_tangible,
  countable_noun->can_be_counted, uncountable_noun->impossible_to_count) to the
  already-shipped three-row table (common_noun/collective_noun/abstract_noun),
  all from the SAME already-cited Grammarly "Nouns: Definition and Examples"
  page -- this file's own header already noted the source gives clean
  single-sentence definitions for common, proper, concrete, abstract,
  collective, singular, plural, countable, uncountable, and gerund nouns, but
  only three had been turned into rows. `concrete_noun` deliberately pairs
  with the already-shipped `abstract_noun` (perceived by the senses vs. not),
  and `countable_noun`/`uncountable_noun` form their own natural pair.
  WebFetch-verified before adding. Three OTHER candidates from the same page
  were deliberately excluded for bundling two distinct facts into one
  sentence rather than stating a single clean fact: `proper_noun` ("...is a
  specific name of a person, place, or thing AND is always capitalized" --
  naming function + a separate capitalization rule), `singular_noun`/
  `plural_noun` ("...refers to one/more than one person, place, thing, or
  idea AND requires a singular/plural verb" -- referent + grammatical
  agreement rule), and `gerund` ("...a verb form that ends in -ing AND
  functions as a noun in a sentence" -- morphological fact + syntactic-
  function fact). Extended e2e test `facts_nountype_e2e.rs` (now 6 tests:
  direct recall and reverse binding for both an original row and a newly
  added row, honest abstention on the pre-existing untabled term, and honest
  abstention on a newly-identified bundled-fact candidate).
