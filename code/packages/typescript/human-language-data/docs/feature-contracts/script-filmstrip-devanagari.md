# Script filmstrips: Devanagari

Devanagari has no table on purpose: its twelve cited signs (ा ु ू े ं ़ ् ृ ँ
ी ो ः; the last three drawn without the piece of headline the font prints
on them, which is the word's headline) come from native writers who wrote
each sign alone, which says nothing about
its order against a consonant or the headline, so a lesson whose headword
is the bare sign prints a one-glyph strip, and कि stays refused. A
Devanagari WORD is one composed entry (its letters' bodies, then one shared
headline), and the one sign it may hold is ā straight after a consonant,
whose place is cited separately (`HEADLINE_WORD_SIGNS`). A phrase of such
words separated by single spaces (`headlinePhraseOf`, at most
`MAX_PHRASE_WORDS` words) prints as a strip of words, "Word 1 of 2", each
word with its own headline.
