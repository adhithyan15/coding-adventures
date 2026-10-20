## Fixed — chapter 1's payoff is atom-scored

Chapter 1's lessons, the ciao writing runway included, are typed (schema v2),
but `chapters.d/0001.json` still carried the schema-v1 placeholder
`payoff.assesses: []` and a note waiting "until migration". The
representativeness gate scored IT-C01-practice 0/20, which was the only
chapter debt keeping the track off the clean list (#12301, #12088).

- The payoff now names the twenty atoms IT-C01-practice assesses — every atom
  the chapter introduces. The IT-W01 writing lessons introduce none of their
  own (they re-assess ciao's three), so the opener's writing evidence is
  already inside the set.
- Italian is now a clean track under the chapter gates. No lesson, book or
  narration changed.

