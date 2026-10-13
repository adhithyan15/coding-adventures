# A tracing lesson as a source (Gurmukhi)

Gurmukhi (`strokes/gurmukhi.ts`) cites the Alphabet Tracing lesson of GNPS's
Gurmukhi Sikho app (Apache-2.0). For each letter, the developer entered ordered
checkpoints over the Noto Sans Gurmukhi letter, one list per pen-down stroke,
and the app makes a child reach them in order. That gives the order, the start,
the direction and the lifts; the record cites them and copies no coordinate.
The path is fitted to the bundled outline. The source draws the headline first,
and every record says this is a teaching order: fluent writers are often
described as adding the headline last. ਛ, ਨ and ਬ lift once less than the app,
down to Omniglot's copyist mode, which is an upper bound on native lifts.

A Gurmukhi WORD of bare cited letters (ਪਰ, ਅਮਨ, ਮਨਨ) is composed like a
Devanagari one: each letter's body without its own (first) headline stroke,
then ONE headline last, left to right. That word-level order is this book's
documented convention, not something the source records, and the strip's
footer says so. ਅ ਖ ਘ ਪ ਮ, whose printed bar is split, join whole, and the
shared headline never crosses their gap (see `headline-word.ts`).
