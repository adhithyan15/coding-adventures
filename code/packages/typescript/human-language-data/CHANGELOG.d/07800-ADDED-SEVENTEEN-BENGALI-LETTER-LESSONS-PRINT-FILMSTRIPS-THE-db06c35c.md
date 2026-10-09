### Added — seventeen Bengali letter lessons print filmstrips, the headline drawn last by convention

- **17 lessons gain a strip** (Bengali 10 -> 27; 831 -> 848 of 1,367 writing
  lessons): BN-W01-ha (হ), BN-W01-ma (ম), BN-W01-na, BN-W01-na-trace,
  BN-W01-na-guided-copy and BN-W01-na-delayed-copy (ন), BN-W02-la (ল),
  BN-W02-ta (ত), BN-W02-ya (য), BN-W03-bha (ভ), BN-W03-da (দ), BN-W04-cha
  (চ), BN-W04-chha (ছ), BN-W04-dda (ড), BN-W04-i-indep (ই), BN-W04-ja (জ)
  and BN-W41-pha (ফ). Each body is cited to native writers' pen traces in HP
  Labs India's LipiTk 4.0 Bangla recognizer (counts and shares only); the
  headline is drawn last by a documented convention, because no headline
  placement wins a majority of those traces.
- **Inventory.** `bengali.json` gives the fourteen letters components, a
  stroke order, a lift count and a cited source whose variation states the
  body counts, the headline placement counts and the convention. Its `notes`
  state the rule's two cases and why ক আ গ ট ধ প স শ ঝ stay uncited.
- **Rewritten:** BN-W41-pha swaps the "copy what you see" disclaimer for the
  numbered-strip wording; BN-W01-na-trace swaps "this book does not tell you
  where each curve of the body starts" for the same. The other fifteen
  lessons carried no disclaimer, and the ন lessons already teach body first,
  bar last, which the strips now show.
- **Pins.** `tests/filmstrip-target-counts/bengali.json` 10 -> 27.
  `bengali.evidence.ts` cites twenty-four rows and pins the fourteen that
  draw the headline last by convention (their variation says so, with counts,
  and their order ends with the headline). `the-real-corpus` lists the
  seventeen lesson targets and the nine undrawn letter lessons.
- **Regenerated:** figures and their hash manifest, the Bengali chapters,
  narration and modality manifests, and the book hashes.
