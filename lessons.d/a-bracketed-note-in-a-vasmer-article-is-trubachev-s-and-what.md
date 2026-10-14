---
category: Testing & coverage
---

# A bracketed note in a Vasmer article is Trubachev's, and what Vasmer reports is not what he holds

An independent review of the 1,148 Russian etymologies paraphrased from Vasmer
found 36 lessons with an error. Every test had passed. Most of the errors fell
into three shapes.

- **Misattributed notes.** The Russian edition closes some articles with
  Trubachev's additions: `[Т]` in the text, and the `trubachev` column in
  tamila-krashtan/vasmer's `articles.csv`. These were cited as "a note by
  Moszyński", "Vaillant, in a note" or "Jakobson, in a note to Vasmer", and
  once simply as "(Vasmer)". In each case the note is Trubachev's, and he is
  reporting or citing that scholar. In RU-C56-forest the note even backs a
  link that Vasmer himself calls doubtful.
- **Reported read as held.** Vasmer often lists what others proposed, then
  gives his own view after "однако вероятнее", "скорее" or "правильнее
  считать". The blocks kept the list and dropped the verdict:
  - серьги: his preferred Old Chuvash source was dropped.
  - грусть: his preferred "gnaw" link was dropped.
  - щи: his own *съто* view was dropped.
  - трогать: his "сомнительно" was moved from *tergeō* onto *trahō*.
- **Slugs coined from the Russian word.** Several root slugs were built from
  the Russian headword rather than from the Proto-Slavic etymon:
  - `slavic-chestu` merged *часто* (\*čęstъ, "dense") with *счастье*
    (\*čęstь, "share"). Vasmer keeps the two apart.
  - `slavic-gor` / `slavic-goreti` split one root.
  - `slavic-dukhu` / `slavic-dykh` split one root.
  - `post-latin` gave inherited *поздно* a Latin etymon.

Do:

- Read the general, origin and `trubachev` fields separately, and credit the
  `[Т]` text to Trubachev.
- For each article, find Vasmer's own preference marker and state it first.
- Before you reuse or coin a slug, check that every lesson already on it has
  the same reconstructed etymon, not just a similar-looking Russian word.
- `check:root-slug-splits` only sees spelling splits (`x`, `x-latin`,
  `latin-x`). It cannot see a merge of two etyma, or a split across two
  differently named Slavic stems.
