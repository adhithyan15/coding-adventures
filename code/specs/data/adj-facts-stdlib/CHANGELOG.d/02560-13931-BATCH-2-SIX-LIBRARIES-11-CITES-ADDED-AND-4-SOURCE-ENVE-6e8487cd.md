- **#13931 batch 2: six libraries, 11 `cites` added and 4 `source` envelopes replaced.** Completes the
  partial-coverage and anaphora work in `meteorology/cloud-type`, `biology/tissue-types`,
  `astronomy/comet-tail-type`, `language/end-punctuation-mark`, `physics/friction-types` and
  `biology/blood-groups`.

  FOUR DISTINCT REMEDIES, which are independent requirements rather than alternatives:

    - ADD per-category evidence where the `source` named only one category.
    - WIDEN past a bare pronoun so the citation names its own subject. `cloud-type` and
      `end-punctuation-mark` had their `source` REPLACED because the existing span WAS the defect —
      a dangling "They often are..." and "It has one job...".
    - MATCH THE ROW'S VALUE, not merely its category. `tissue-types`' connective and muscle rows
      needed sentences naming bone/blood and cardiac/skeletal; a sentence naming only the tissue
      LOOKS like per-row evidence while not supporting the row's actual claim.
    - BOTH AT ONCE — `friction-types`' rolling row needed "spherical object" from a sentence opening
      with a bare "It", so it is widened AND value-matched.

  `tissue-types` draws each of its four sentences from ITS OWN SEER page, so this one file also
  demonstrates the #13934 shape.

  *** TWO LIBRARIES WERE DELIBERATELY NOT TOUCHED, AND ONE WAS DELIBERATELY NOT WIDENED. ***
  `language/vowels` and `biology/plant-life-cycle` are FALSE POSITIVES of the screen that found this
  class — the first because its category atoms are single letters the word matcher drops, the second
  because its rows are ordinal positions rather than categories. And `comet-tail-type`'s existing
  "This dust tail traces..." is FINE: a demonstrative PLUS its noun names the subject; only bare
  pronouns fail. Acting on those three would have been damage, not repair.

  *** A CITATION I WROTE MYSELF WAS FABRICATED, AND ONLY EXTRACTION CAUGHT IT. *** Drafting
  `blood-groups`' widened source I wrote "In 1900, Karl Landsteiner discovered the ABO blood group
  system." from what I knew about Landsteiner. THAT SENTENCE IS NOT ON THE PAGE. The real antecedent
  for its "He" sits TWO sentences earlier, making the verified span 702 characters. The standing rule
  had been "never copy the header quote"; the correct rule is that EVERY citation string must come
  from an extraction, INCLUDING one written while confident of the fact — confidence being exactly
  the condition under which the check gets skipped.

  All six `.query.adj` companions still parse and run. Verified in output: `blood-groups` no longer
  opens with a bare "He", and `cloud-type`'s cirrus answer now carries a source that names cirrus.

