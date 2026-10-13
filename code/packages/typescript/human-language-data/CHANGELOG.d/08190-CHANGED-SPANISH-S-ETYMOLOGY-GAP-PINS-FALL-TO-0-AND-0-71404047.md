### Changed — Spanish's etymology gap pins fall to 0 and 0

`ETYMOLOGY_GAP_PINS.spanish` in `tests/root-ledger.test.ts` drops from `{ withoutHook: 486, withoutVisible: 209 }` to `{ withoutHook: 0, withoutVisible: 0 }`. Every Spanish word lesson now has an `etymology_hook` and states its word's origin in its body.

- **What the lessons gained.** 287 lessons that already told the story gained a hook that agrees with it; 199 gained a "taken apart" block and a hook, 185 of them the generated wave ES-C475 to ES-C511; ES-C52-como-se-dice, at 299 s, gained one origin sentence instead. Sources are EtymDB 2.1 (Fourrier & Sagot 2020, CC BY-SA 4.0), checked row by row and relabelled by Spanish sound change where the dataset called a learned word inherited, and Lewis & Short for Latin; Arabic loans name the Arabic word. About sixteen say the origin is uncertain or only usually traced. The Spanish track changelog lists the 41 corrected older passages and the eleven `max_seconds` raises, all under 300 s.
- **No code changed.** Only the pin and its comment moved. `check:root-slug-splits` reports no new split.
