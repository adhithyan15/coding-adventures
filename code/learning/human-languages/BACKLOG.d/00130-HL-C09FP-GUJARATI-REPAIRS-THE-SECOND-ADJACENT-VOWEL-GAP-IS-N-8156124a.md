## HL-C09FP — Gujarati ઔ repairs the second adjacent vowel gap; ઋ is next

The source-and-font audit found **ઔ** missing from `gujarati.json`. Its
t30apps.com animation repeats the four verified **ઓ** runs—joined body, first
stem, trailing stem, and lower high arc—then adds a fifth, higher arc. The new
inventory entry and Noto Sans Gujarati fit preserve all five paths and four
observed lifts.

Gujarati is now **10/35 verified, 25 remaining**. Rechecking canonical vowel
order exposed another inventory gap at **ઋ**, which appears in both the same
teaching source and bundled font but not the current data. Repair **ઋ** before
starting **ક**; a correctness defect still outranks coverage if one appears
during fitting or validation.

