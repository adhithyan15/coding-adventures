- `language/comma-rule.adj` (new) -- a new `comma_rule(rule, description)` table names three
  comma rules and what each actually says to do
  (comma_in_a_series->use_commas_to_separate_elements_in_a_list_of_more_than_two_elements,
  comma_before_but->use_a_comma_before_but_when_it_is_joining_two_independent_clauses,
  comma_with_direct_address->set_off_the_name_with_commas_when_addressing_another_person_by_name),
  quoted verbatim from Grammarly's "Rules for Using Commas, With Examples" article -- `trust
  consensus`, the same tier the sibling `end-punctuation-mark.adj`/`capitalization-rule.adj`
  already use for their Grammarly citations. This table is the natural complement to
  `end-punctuation-mark.adj`, which explicitly named comma as a real mark it deliberately
  excludes because it belongs to a different category (a mid-sentence pause mark, not an
  end-of-sentence mark) -- this table now grounds that mid-sentence category on its own
  terms. Picked using the mandatory full-tree-grep-before-scoping discipline -- zero hits
  for `comma_rule|oxford_comma|direct_address` before writing. WebFetch-verified twice --
  the second pass pulled the full paragraph under each of the three chosen headings,
  confirming each quoted sentence is the article's own first, complete, standalone rule
  sentence, with worked examples following it rather than folded into the quote. Honest
  abstention on `oxford_comma`: a real, well-known term the same page discusses under its
  own heading, but its own rule sentence bundles the placement rule together with a caveat
  about its optionality rather than stating one clean single fact. New manifest objective
  `adj.literacy.k2.comma_rule` (band K-2, `recall` competency, `ccss.ela` coverage root).
  New e2e test `facts_commarule_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention on an untabled term).
