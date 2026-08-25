### Added - the metalanguage ramp (HL-C89)

The hidden prerequisite of every language textbook: it assumes the reader
already knows grammar *vocabulary*. "The first-person singular present
indicative of a regular -ar verb" spends six technical terms on one form, and a
beginner who never studied grammar understands none of them. The book is gentle
about Spanish and brutal about English, and nobody notices, because the author
has known those words since school.

`core/metalanguage.json` makes it a ramp: **54 terms**, each carrying the thing
the learner must already be able to DO before the term is named. `verb` arrives
once *soy*, *estoy* and one present form are in use. `mood` waits for block D of
the subjunctive arc, twenty-four lessons in.

**`plainAlternative` is the point.** A rule that only forbids is a rule authors
route around, so every term carries what a lesson must say *instead* until the
term is earned -- "a doing word" for verb, "the plain form, the one in a
dictionary" for infinitive, "whether you are asserting or wanting" for mood. The
gate can tell an author what to write, not merely what not to.

The first measurement: **2,289 technical uses across 1,161 lessons**, led by
`verb` (795 lessons), `noun` (398), `regular` (109), `tense` (109), `pronoun`
(102), `article` (91). Nothing anywhere introduces any of them.

**Two numbers, deliberately.** The raw total is 7,738, but `word` alone appears
in 1,555 lessons and needs no introduction at all. A measurement that does not
separate ordinary English from technical vocabulary produces one enormous number
that is identical for every corpus and useless to every author -- the same
cry-wolf failure the info-dump gate avoided by flagging shape rather than size.
So terms carry `technical`, the total says how pervasive the assumption is, and
the technical count says what to fix first. `noun`, `verb` and `adjective` count
as technical on purpose: the premise is a reader who never studied grammar, and
for them "a doing word" lands and "verb" does not.

