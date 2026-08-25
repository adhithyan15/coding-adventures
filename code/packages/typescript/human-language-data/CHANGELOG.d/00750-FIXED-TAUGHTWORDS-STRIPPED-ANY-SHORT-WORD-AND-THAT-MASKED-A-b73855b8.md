### Fixed — `taughtWords` stripped any short word, and that masked a second bug

A headword carries its article — *el pan*, *la casa* — and a body saying *bebo agua*
is using the same word, so the bare noun has to match too. The rule that did this
stripped **any leading word of three characters or fewer**.

A census of the corpus's own headwords shows what that meant: the rule fired on
**227 of 1,453 lessons** (247 headword parts), and only **49 lessons** (64 parts)
actually begin with an article. It was registering

- `llamo` as taught by *me llamo*, `favor` by *por favor*, `dia` by *bom dia*,
- `piace` by *mi piace*, `heiße…` by *ich heiße…*, `wiedersehen` by *auf wiedersehen*,
- and the night- and afternoon-word of every ശുഭ / शुभ / శుభ / ಶುಭ greeting across
  Malayalam, Hindi, Telugu and Kannada — because all those openers are three
  characters.

The rule is now an **allowlist of real definite articles, per language**, taken from
that census rather than from a length guess. Two deliberate exclusions: Spanish
**`lo`** (it *is* a neuter article, but the corpus's only `lo `-headword is *lo
siento*, where it is a pronoun) and Italian **`a`** (*a domani*, *a presto* are
prepositions). A track absent from the map never has anything stripped, which is right
for Latin, Arabic, the Indic tracks and every other language whose headwords carry no
article — Arabic's `ال` is prefixed without a space, so neither rule ever reached it.

