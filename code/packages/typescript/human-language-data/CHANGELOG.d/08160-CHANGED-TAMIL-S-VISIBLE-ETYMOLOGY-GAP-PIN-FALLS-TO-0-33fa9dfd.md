### Changed — Tamil's visible-etymology gap pin falls to 0

`ETYMOLOGY_GAP_PINS.tamil` in `tests/root-ledger.test.ts` drops from `{ withoutHook: 0, withoutVisible: 4 }` to `{ withoutHook: 0, withoutVisible: 0 }`. Every Tamil word lesson now states its origin in its body, not only in its hook.

- **The four.** TA-C06-dative-subject, TA-C37-ivar, TA-C37-ivar-en-nanbar and TA-C39-evvalavu estimated at 294–299 s, so a full "taken apart" block would have gone over 300 s. Each gained one origin sentence in an existing section, such as "a system inherited from Dravidian (DEDR 410)" and "*Enakku* and *teriyum* are Dravidian (DEDR 5160, 3419)". Redundant prose was trimmed to pay for it. Only TA-C39-evvalavu needed a raise (294 → 298 s). All four stay under 300 s.
- **Review.** The same change is a full accuracy review of the 1,200 Tamil etymology blocks and hooks. It fixes one wrong DEDR entry, 12 misreported sources, five false "no entry" claims, and 74 lessons that had dropped DEDR's Sanskrit or Indo-Aryan comparisons. The Tamil changelog lists them.
- **No code changed.** Only the pin and its comment moved.
