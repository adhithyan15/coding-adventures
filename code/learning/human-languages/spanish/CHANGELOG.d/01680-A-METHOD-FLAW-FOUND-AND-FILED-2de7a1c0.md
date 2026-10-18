## A method flaw found and filed

This container starts with **`LC_ALL` and `LANG` unset**, so grep runs in the C
locale and `.` matches **one byte**. In UTF-8 the ñ in *acompañar* is two, so

```sh
grep -rln "^headword:.*acompa.ar" *.md   # 0 hits
```

returned zero for a word the corpus teaches with a full lesson
(`ES-C446-acompanar`). A false zero in the pre-check is expensive — it is
exactly how a duplicate lesson gets written.

Worse, the failure is inconsistent: `compa.ero` returned four files because
lesson **ids** use ASCII spellings, so the pattern matched `companero` in an id
while missing *compañero* in prose. Filed in `lessons.d/`: type the accented
character, and confirm any zero a second way. Verified afterwards that **no**
word declared absent in chapters 468–472 is accented, so no earlier conclusion
rests on the flaw.

