---
category: Testing & coverage
---

# A font's ligature list is not the list of pairs it reshapes; contextual lookups fuse and reshape too

**Context:** teaching the filmstrip composer (`human-language-data`'s
`writtenPiecesOf`) to draw Gujarati words as consonant, then vowel sign. A pair
the font joins into one shape must be refused, because drawing the consonant
and the sign apart would draw a word nobody writes.

**What happened:** the refusal list was first built from the GSUB *ligature*
lookups of Noto Sans Gujarati (type 4: ણુ, રુ, રૂ, જા, જી). That missed two
things the font also does, both through *contextual* lookups (types 5 and 6):

- after જ and ૹ, a chained lookup in `psts` splits ો and ૌ into ા plus ે or
  ૈ, and the ા then ligates with જ. So જો prints with the bar joined, and the
  composed strip for the lesson GU-C35 (જો) drew something the page does not
  show. It had already been generated, and the count pin moved to match it.
- in `blws`, a contextual lookup swaps 22 consonants (ખ ગ ઘ ... સ) for "stem"
  forms of their own before ુ and ૂ: the printed consonant grows a stem that
  neither its own ductus nor the sign's draws.

**Fix:** walk every GSUB lookup type (4, 5, 6 and 7 extensions), list the
glyphs each contextual subtable covers, and follow its nested lookups to see
what it substitutes. The Gujarati entry of `FUSED_SIGN_PAIRS` now holds all 54
reshaped pairs, and GU-C35 stays undrawn.

**Do differently:** before declaring which consonant-sign pairs a font leaves
alone, dump every lookup that mentions the sign glyphs, contextual ones
included, and read what each one substitutes. A pair that has no ligature entry
can still be fused or reshaped by context.
