- `language/prefix-meaning.adj` (extended) -- adds three more prefixes
  (non_->not_or_negation, pre_->happening_before, inter_->among_between) to the
  already-shipped three-row table (un_/re_/dis_), all from the SAME already-cited
  Grammarly "Prefixes: Definition and Examples" page, which turns out to define
  roughly seventy prefixes as clean, single-fact short phrases in its own table --
  only three had been used so far. Picked for genuinely distinct semantic categories
  rather than near-synonyms of the negation family already covered: `non_` (negation,
  distinct word from un_/dis_), `pre_` (temporal -- before), `inter_` (relational --
  among/between). WebFetch-verified twice. This cycle also researched and abandoned a
  `homograph` candidate -- every source tried (7ESL, general web summaries) bundles
  both meanings of a homograph word into one comparative sentence rather than stating
  one clean meaning per sentence -- and an `interjection-type` candidate, dropped
  after a secondary source's own "cognitive interjection" definition muddled with its
  "emotive" category, an editorial-quality red flag. `over_` remains the abstention
  target (a real prefix the same source page also covers, but still deliberately not
  a row). Extended e2e test `facts_prefixmeaning_e2e.rs` (now 5 tests: direct recall
  and reverse binding for both an original row and a newly added row, honest
  abstention).
