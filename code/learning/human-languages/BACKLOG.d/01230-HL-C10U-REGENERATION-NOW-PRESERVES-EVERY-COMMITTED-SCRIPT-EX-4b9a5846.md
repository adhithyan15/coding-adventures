## HL-C10U — regeneration now preserves every committed script extension

While adding **ఎ**, a direct run of `generate_syllabary.py` exposed a fail-open
generator boundary: it preserved only the exceptions hard-coded in its own
small map and erased newer verified Kannada, Malayalam, and Telugu rows from
the generated JSON. Script-ductus immediately failed because Kannada **ಅ** no
longer had a verified source. The generated files were restored before that
change.

The generator now treats the committed JSON as the authoritative merge boundary
for hand-authored extensions. Unicode still rebuilds glyph identity, sound, and
role, while every other field survives; downstream collections such as marks
and Malayalam final consonants survive wholesale; core-external rows survive;
and malformed or duplicate glyph identities fail closed. Six focused Python
regressions include a full semantic idempotence check across all three scripts,
and a real regeneration leaves their tracked content unchanged. Urdu **پ** is
again the next measured glyph at **10 affected realizations**.

