---
category: Repo policy / workflow reminders
---

# A family label is not a direct owner when its members have different policy identities

The first HL47 split put Arabic, Perso-Arabic, and Urdu Nastaliq activation in
one `arabic-family.ts` file. They currently share the same conservative
isolated-letter behavior, but they have different script ids, tracks, fonts,
and future evidence. A file named for the family would make the first
script-specific change a shared edit again.

Use one direct source owner per actual policy identity. Sharing an algorithm is
fine; sharing a policy file is not merely because today's objects have the
same shape. Structural ownership tests should name each direct owner so a
future family bucket cannot silently return.
