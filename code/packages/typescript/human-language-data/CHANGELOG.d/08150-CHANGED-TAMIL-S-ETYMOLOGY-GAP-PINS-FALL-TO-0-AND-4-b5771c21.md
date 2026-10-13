### Changed — Tamil's etymology gap pins fall to 0 and 4

`ETYMOLOGY_GAP_PINS.tamil` in `tests/root-ledger.test.ts` is lowered. Word lessons without an `etymology_hook` fall from 1185 to **0**, and lessons without visible etymology fall from 1144 to **4**. Every Tamil lesson that introduces a word now says where that word comes from (HL24 §9).

- **Sources.** Native words cite the *Dravidian Etymological Dictionary* (Burrow & Emeneau, DEDR). Sanskrit and Indo-Aryan loans cite Turner's CDIAL. Both are cited by entry number and were read locally from the moli-mandala/data (Jambu) files. Every cited number was checked against those files. No dataset file is vendored, and no source text is copied: each statement is our own sentence.
- **Shape.** Each lesson that had no visible etymology gained a `## The word, taken apart` block (or `## The phrase, taken apart`) after the introducing block. Each also has a matching `etymology_hook` of 120 characters or fewer. The 56 lessons whose body already stated an origin gained only the hook.
- **Answers.** 874 native Dravidian, 158 Sanskrit or Indo-Aryan loans, 43 other loans with the donor named, 65 built from known parts, and 60 uncertain. Each uncertain one gives the usual account with a hedge. The added text has 2,430 DEDR citations and 352 CDIAL citations.
- **The four left.** TA-C06-dative-subject, TA-C37-ivar, TA-C37-ivar-en-nanbar and TA-C39-evvalavu already estimate at 294–299 s. Any visible block would push them past the 300-second limit, so only their hooks carry the etymology.
- **Ledger side effects.** 1,148 Tamil lessons gained `roots:` slugs: 872 distinct slugs, 91 reused and 781 new. `check:root-slug-splits` reports no new split. Twenty-seven lessons had `duration.max_seconds` raised to their computed length, all under 300 s. The Tamil changelog lists them.
- **No code changed.** Only the pin and its comment moved.
