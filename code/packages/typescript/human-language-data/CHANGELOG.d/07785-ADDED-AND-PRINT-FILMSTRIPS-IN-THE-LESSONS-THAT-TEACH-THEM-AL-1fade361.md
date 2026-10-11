### Added — ी, ो and ः print filmstrips in the lessons that teach them alone

- **11 lessons gain a strip** from the new script-ductus entries: Hindi
  HI-S116-vowel-sign-ii, HI-W12-ii-matra (ी) and HI-S150-vowel-sign-o (ो);
  Marathi MR-W01-ii-matra (ी), MR-W01-o-matra (ो) and MR-W02-visarga (ः);
  Marwadi MW-W04-ii-matra (ी) and MW-W05-o-matra (ो); Sanskrit
  SA-S210-vowel-sign-ii (ी), SA-S214-vowel-sign-o (ो) and
  SA-S201-sign-visarga (ः). Each draws the sign alone; no word or
  consonant + sign headword changes (Devanagari still has no
  `WRITTEN_SIGN_SIDES` row, and the word composer still takes only ā).
- **The grey stub.** The printed sign carries a short piece of headline the
  strip leaves undrawn: in a word it is the headline, which native writers
  draw last across the whole word. The mark records and script-ductus's
  coverage exception say so.
- **Pins.** `tests/filmstrip-target-counts`: hindi 58 -> 61, marathi
  50 -> 53, sanskrit 51 -> 54, marwadi 41 -> 43. `the-real-corpus` lists the
  eleven lessons among the bare-sign targets and now holds the ि, ै and ौ
  lessons (HI-W128, MR-W05-i-matra, SA-S109, HI-S134, HI-W12-ai-matra,
  MR-W56, SA-S223) as the ones left undrawn. `devanagari-signs-drawn-alone`
  cites twelve signs, checks that the three stub-carrying records explain
  the stub, and that ि ै ौ have no stroke source.
- **Regenerated:** figures and their hash manifests, the five rewritten
  lessons' chapters, narration and modality manifests, and the book hashes.
