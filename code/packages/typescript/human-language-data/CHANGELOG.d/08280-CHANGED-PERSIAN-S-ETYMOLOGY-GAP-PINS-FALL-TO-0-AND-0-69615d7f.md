### Changed — Persian's etymology gap pins fall to 0 and 0

`tests/etymology-gap-pin-owners/persian.owner.ts` drops from `{ withoutHook: 1300, withoutVisible: 1311 }` to `{ withoutHook: 0, withoutVisible: 0 }`, with a factual rationale in place of the migration note. Every Persian word lesson now has an `etymology_hook` and states its word's origin in its body.

- **What the lessons gained.** 1,300 vocabulary-wave lessons gained a "taken apart" block and a hook; 12 hand-written lessons gained a block; 14 hooks that described grammar or spelling instead of origin were replaced. Sources are EtymDB 2.1 (Fourrier & Sagot 2020, CC BY-SA 4.0) for Middle Persian forms and loan donors, each cited form checked against the EtymDB rows by script, and Lane's Lexicon for Arabic roots, every root checked by script. Forms no source read here confirms are hedged as "usually traced to"; about 80 origins are explicitly not settled or only partly settled, after an independent review of all 1,320 changed lessons fixed 54 (4.1%) and gave 33 earlier "not settled" origins a hedged, sourced account. The Persian track changelog has the detail, including the older claims corrected.
- **Empty etymology block pin 2 → 0.** `EMPTY_ETYMOLOGY_BLOCK_PINS.persian` in `tests/root-ledger.test.ts` falls to 0.
- **No code changed.** Only test pins and the owner's rationale moved. No `max_seconds` was raised.
