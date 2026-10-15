# Script filmstrips: the Latin family

Spanish and German followed Punjabi, the first Latin-script tracks, and
French, Italian, Portuguese and Latin followed once the Latin strips moved
to an outline with a one-storey a. `data/scripts/latin.json` holds exactly
the Latin characters the six Latin-script tracks' headwords use (NFD), plus
the precomposed ñ á é í ó ú ü that are drawn whole, and 31 of its rows cite
the Grundschrift-App or UJIpenchars2's native Spanish writers. Its font is
`_fonts/LatinPrint-Subset.ttf`, a renamed subset of SIL's literacy typeface
Andika: Noto Sans prints a two-storey a, and every source teaches the
one-storey a, so the strip shows the handwriting model's letter while the
book's text keeps its own typeface. `latin` is in `SEPARATE_LETTER_SCRIPTS`,
since print letters stand apart, so 26 lessons print a strip (hola, salut,
ciao, olá, Hallo, quia, ayer, buenos días, parce que, weil, ñ, ß, "¿ ¡" and
"á é í ó ú"). Lessons with an uncited mark (è ê ç ä ö ë ï ē, œ),
punctuation inside a word, or more than `MAX_SEQUENCE_PIECES` pieces stay
undrawn.
