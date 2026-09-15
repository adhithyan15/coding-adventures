- `language/comparative-adjective-rule.adj` (new) — a `table` naming five common English
  comparative-adjective ("-er") formation rules and what each one actually requires:
  `comparative_adjective_rule(rule, description)`, `one_syllable_adjective` →
  `add_er_suffix`, `one_syllable_adjective_ending_in_e` → `add_r_only`,
  `one_syllable_consonant_vowel_consonant` → `double_final_consonant_before_er`,
  `adjective_ending_in_y` → `change_y_to_i_before_er`,
  `two_syllable_adjective_ending_in_er_ow_or_le` →
  `add_er_or_r_without_spelling_change`. This is the "-er" sibling to
  `superlative-adjective-rule.adj` (which covers only "-est" formation), closing a
  genuinely-uncovered-anywhere-in-this-stdlib gap flagged across several prior session
  threads. Deliberately a NEW, distinct predicate rather than an extension of either
  neighboring table: `superlative-adjective-rule.adj` states its own "-est" rules only, and
  `suffix-meaning.adj` separately ships the AGENTIVE `-er` sense (`_er_agentive` → "one who;
  person connected with", as in "teacher") — a suffix-MEANING fact, categorically different
  from this comparative-degree GRAMMAR rule about adjective spelling. Using a new predicate
  name means there is no atom-label collision to disambiguate in the first place (unlike the
  `ow`/`ow_long_o` and `_er_agentive`/bare-`_er` cases earlier in this stdlib) — this table
  freely ships the bare atom `_er`-shaped rule names without needing any special-cased
  disambiguated label. Quoted verbatim from Grammarly's "What Are Comparative Adjectives?
  Definition and Examples" article, "5 spelling rules for forming comparative adjectives"
  section — curl-fetched and read byte-for-byte before writing this file, confirming all five
  quoted sentences appear verbatim in the source. Honest abstention on the source's own sixth
  rule (long two-or-more-syllable adjectives use "more" instead of "-er"): its own supporting
  text is a bullet-list fragment rather than a clean quotable sentence, the same reason
  `superlative-adjective-rule.adj` itself excludes its analogous "most" rule — so a recall on
  it ABSTAINS rather than inventing a description. Empirically verified all five rows (forward
  and reverse) plus the honest abstention against the real built `adj-lang-cli` binary in a
  scratch table before writing the shipped file. New `comparative-adjective-rule.query.adj`
  and `facts_comparativeadjectiverule_e2e.rs` (6 tests: direct recall, reverse recall, two
  tests dedicated to the two rules that have no `-est`-formation counterpart in the sibling
  table, the CVC-doubling rule, and the honest abstention); new manifest objective
  `adj.literacy.3to5.comparative_adjective_rule` added via a surgical text edit (verified with
  `git diff --stat`, JSON validated before write).

