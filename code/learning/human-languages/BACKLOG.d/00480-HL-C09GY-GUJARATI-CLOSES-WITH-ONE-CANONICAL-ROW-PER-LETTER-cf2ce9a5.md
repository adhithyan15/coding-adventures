## HL-C09GY — Gujarati closes with one canonical row per letter

The completion audit found that restored **ન** and **પ** still had their old
conventional placeholders later in `gujarati.json`. Those duplicate rows made
the inventory appear to contain 46 letters even though the teaching-app
sequence and the font-backed ductus cover 44 unique letters. The stale rows are
removed, and a cross-script uniqueness gate now rejects this class of drift.

Gujarati is now **44/44 source-verified and complete**. Future work should treat
this track as maintenance: repair source, font-fit, rendering, or curriculum
defects when evidence finds one, rather than manufacturing a 45th letter.
The deduplicated Gujarati-bearing `script-data` batch measures **82.13 kB**,
below its 250 kB target.

