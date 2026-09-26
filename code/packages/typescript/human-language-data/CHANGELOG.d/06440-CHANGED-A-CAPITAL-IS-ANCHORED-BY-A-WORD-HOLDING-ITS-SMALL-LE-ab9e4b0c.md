### Changed — a capital is anchored by a word holding its small letter (HL-C443)

`letter-anchoring.ts` now treats a case pair as one letter in two forms.
A lesson that writes a capital, such as Russian Я, counts as anchored once an
earlier word holds its small letter: here я, "I", from chapter 2. This matches
the voiced-kana rule, where が is anchored by か and ゛.

The pairing is one-to-one only. A letter whose other case is more than one
code point, or that has no case at all (every Indic, Arabic and kana letter),
is treated as itself. Only anchoring reads case pairs. The completeness count
(`unwritten`) still asks for each form a word shows.

Measured effect: Russian cold goes 1 → 0. Every other track is unchanged.
Russian's small я keeps its own lesson in chapter 28.
