## 0.6.1 — Tamil and Gujarati join the script list

- **Tamil** is new: `data/scripts/tamil.json` ships with the first handwriting
  lessons for **any Dravidian language** (`TA-W01`–`W04`). 11 letters and 4
  marks, `complete: false`.
- **Gujarati was already there and simply never wired in.** `gujarati.json` has
  existed since the Gujarati track was authored, but `SCRIPTS` in `src/data.ts`
  listed five scripts while `data/scripts/` held six. Both are now included, so
  Browse and Practice cover **seven** scripts.
- No logic changed — two imports and two array entries. `tests/core.test.ts` uses
  `arrayContaining`, so the script list is not pinned by count.

