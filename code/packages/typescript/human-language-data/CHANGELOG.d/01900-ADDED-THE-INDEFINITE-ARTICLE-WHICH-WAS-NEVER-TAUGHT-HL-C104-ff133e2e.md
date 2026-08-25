### Added - the indefinite article, which was never taught (HL-C104)

- `un` and `una` did not exist anywhere in the corpus. A learner reaching
  chapter 68 held 27 concrete nouns -- colours, family, body parts, food,
  animals -- and could say *the* dog but not *a* dog.
- HL10 §5.4 rung 3 says "definite article, **then indefinite**", and §12.2 block
  11 says "four lessons, **then indefinite**". The definite articles shipped;
  the indefinite ones never followed.
- Now chapter 3, immediately after the definite articles and the agreement
  payoff, where the learner already holds gender and three nouns: `un`, `una`,
  and a review that lines up all four articles.
- The etymology is a gift. **`un` IS the number one** -- Latin *unus* -- and
  Spanish never separated them, so *un dia* is *a day* and *one day* at once.
  English made the identical move and hid it better: *a*/*an* is the word *one*
  worn down, which is why *an hour* and *one hour* begin with the same sound.
  Spanish left the seam visible.
- Fixes seven book targets whose filenames disagreed with their chapter numbers.
  The early chapters use a zero-padded `ch03-` prefix and the renumber regex
  looked for `/ch3-`, so those targets kept a stale filename through several
  renumbers -- surfacing only now as "book chapter 3 occurs twice".
- Spanish 91 -> **92 chapters**, 226 lessons. Fully drivable chapters
  **372 -> 373**.

