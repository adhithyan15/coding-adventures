## HL-C10U — regeneration must preserve every verified script exception

While adding **ఎ**, a direct run of `generate_syllabary.py` exposed a fail-open
generator boundary: it preserved only the exceptions hard-coded in its own
small map and erased newer verified Kannada, Malayalam, and Telugu rows from
the generated JSON. Script-ductus immediately failed because Kannada **ಅ** no
longer had a verified source. The generated files were restored before this
change, and **ఎ** is now represented in both generator intent and committed
data, but the wider preservation gap remains.

Before treating that generator as a safe whole-file rewrite, move every
verified exception behind one authoritative merge boundary or make the script
fail closed when regeneration would remove sourced rows. This infrastructure
repair outranks the next measured glyph because an innocent regeneration can
silently reopen already closed provenance work.

