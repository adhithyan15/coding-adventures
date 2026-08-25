## HL-C215 — two pre-existing mixed-script typos in Hindi, reported not fixed

A per-word purity sweep of all 485 files in the Hindi tree found exactly two, both
shipping into the PDF and the narration:

```
HI-C33-shubh-dopahar.md:40   "the widened सense"   DEVANAGARI SA standing in for Latin s
HI-C32-shubh-sandhya.md:54,76 "रात्रि became राat"  should be रात
```

Both render as plausible wrong text — the same class as the Telugu and Malayalam
defects already fixed under HL-C202, and invisible to any check that looks at one
script at a time.

**Left unfixed deliberately.** They sit in chapters unrelated to the tranche that
found them, and touching them would pull those chapters' book and narration hashes
into an unrelated diff. They want their own small change.

Not a defect, do not "fix": `HI-C25-din.md:56` carries Proto-Slavic `*dьnь`, whose
Cyrillic yers inside a Latin transliteration are correct scholarly notation.

