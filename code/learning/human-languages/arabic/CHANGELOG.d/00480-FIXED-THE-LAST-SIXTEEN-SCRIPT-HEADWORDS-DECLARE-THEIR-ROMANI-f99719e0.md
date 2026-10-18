## Fixed — the last sixteen script headwords declare their romanization

The script-closure report's `headwordsWithoutRomanization` counted sixteen Arabic
lessons whose headword is in Arabic script with no `romanization:` (16 -> 0). Each
now declares the value its own title, body or review lesson already gave:

- AR-C01-al — *al-*; AR-C02-ii-my — *-ī*; AR-C02-ism — *ism*;
  AR-C02-ismii — *ismī …*; AR-C02-maa — *mā*; AR-C05-naam — *naʿam*;
  AR-C05-la — *lā*; AR-C11-ras — *raʾs*; AR-C25-yawm — *yawm*;
  AR-C26-layl — *layl*.
- AR-C08-al-jumua-as-sabt — *al-jumʿa as-sabt*, AR-C10-ab-umm — *ab umm* and
  AR-C10-akh-ukht — *akh ukht*: one space between the words, as
  AR-C08-al-ahad-al-khamis and the AR-C09 colour lessons do.
- AR-C16-al-saa — *as-sāʿa*, with the sun-letter assimilation the lesson's own
  *kam as-sāʿa?* uses.
- AR-C02-maa-ismuka — *mā ismuka* and AR-C17-kam-umruka — *kam ʿumruka*: the
  masculine citation form without the question mark, as AR-C03-kayfa-haluka's
  *kayfa ḥāluka* is. The title's *mā ismuka / ismuki* is not used because chapter 2's
  narration already writes "(mā ismuka)" after the phrase, and a longer value would
  have been paired a second time beside it.

Every headword glyph was already taught by an earlier script lesson, so closure
violations (0), `exposureExemptedGlyphs` and `exposureOnly` are unchanged. The book's
section short titles (contents and PDF bookmarks) in chapters 1, 2, 5, 8, 10, 11, 16,
17, 25 and 26 now pair script with romanization, and the narration pairs these
headwords where chapter text mentions them without one. In chapter 2's narration a
standalone ي now carries both readings, "(yāʾ)" and "(-ī)", because
AR-W05-ya-and-my-name romanizes the same headword *yāʾ* and the chapter-wide pairing
applies each lesson's value.

