#!/usr/bin/env python3
"""Tests for dict_lint — the dictionary enforcement linter.

Run: python3 test_dict_lint.py   (exits non-zero on failure)
"""
import dict_lint

FIND, HYP = dict_lint.load_dictionary()


def check(name, text, expect_clean, expect_substr=None):
    viol = dict_lint.lint(text, FIND, HYP)
    clean = not viol
    assert clean == expect_clean, f"{name}: expected clean={expect_clean}, got {viol}"
    if expect_substr is not None:
        assert any(expect_substr in v for v in viol), f"{name}: {expect_substr!r} not in {viol}"
    print(f"ok   {name}")


# a valid rulebook-style snippet (findings + hypotheses + citations + comments)
check("valid_rulebook", '''
% bacterial arm
prior 0.037 for bacterial_meningitis
  source "Nigrovic 2007 JAMA" trust authoritative
contributes 85 from csf_gram_stain(positive) to bacterial_meningitis
  source "WHO 2025" trust consensus
interacts 0.3 when csf_glucose(low) and csf_lactate(elevated) for bacterial_meningitis
? bacterial_meningitis
? viral_meningitis
''', expect_clean=True)

# a valid case-style snippet (observe + query)
check("valid_case", '''
observe csf_neutrophilic_pleocytosis(high)
observe csf_glucose(low)
observe enteroviral_pcr(positive)
? bacterial_meningitis
''', expect_clean=True)

# unknown finding functor
check("unknown_functor",
      "observe csf_color(yellow)\n? bacterial_meningitis",
      expect_clean=False, expect_substr="unknown finding functor: csf_color")

# value outside the functor's domain
check("bad_value",
      "observe csf_glucose(sky_high)\n? bacterial_meningitis",
      expect_clean=False, expect_substr="not in domain of csf_glucose")

# unknown hypothesis
check("unknown_hypothesis",
      "contributes 2.0 from seizure(present) to fungal_meningitis\n? fungal_meningitis",
      expect_clean=False, expect_substr="unknown hypothesis: fungal_meningitis")

# comments and citation strings must not trip the scanner (csf_color only inside a string)
check("string_and_comment_ignored",
      'observe csf_glucose(low)  % note: not csf_color(yellow)\n'
      '  source "mentions csf_color(yellow) in prose"\n? bacterial_meningitis',
      expect_clean=True)

print("\nall dict_lint tests passed")
