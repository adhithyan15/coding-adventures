- **#15139: five locators that no longer addressed their evidence.** A `locator` exists so a reader
  can re-check the span beside it. I audited all **315 unique locators** in this stdlib (85 hosts) on
  2026-09-13; 271 resolve, and this entry fixes the five that had moved.

  ### The control is the whole design

  `www.myplate.gov` answers **every** path — including `/zzz-adj-locator-control-847…` — with a 301 to
  its own root. From a single request, "this page was removed" and "this host redirects everything"
  are the same observation. So each host is probed with a nonsense path **first** and classified
  (`strict` = it 404s on nonsense, so a 404 elsewhere means something; `softroot` / `soft200` /
  `blocked` = its status codes prove nothing), and no URL's verdict is stronger than its host's
  control allows. Without that, this entry would have claimed dead pages it cannot demonstrate are
  dead.

  | verdict | n |
  | --- | --- |
  | OK | 271 |
  | UNVERIFIABLE (host soft-404s or blocks) | 15 |
  | BLOCKED (403/429 to a scripted client) | 12 |
  | MOVED, same page under a canonical address | 7 |
  | **MOVED to a different path** | **5** |
  | REDIRECTED_TO_ROOT (`myplate.gov`) | 3 |
  | DEAD (404 on a host that 404s honestly) | 2 |

  ### What changed here — refreshed, not re-grounded

  12 occurrences across 8 files: `nei.nih.gov/learn-about-eye-health/…` → `/eye-health-information/…`;
  three USGS Water Science School paths that lost their `/special-topics/` prefix (and, for the water
  cycle, its `/science/` segment too); and `whoi.edu/know-your-ocean/…` → `/ocean-learning-hub/…`.

  **Every span shipped under a stale locator was checked against the destination before the URL was
  touched** — a redirect proves the host still answers, not that the new page still carries the
  quoted sentence. All twelve spans are present: **eight occur once, four twice** — the aquifer
  sentence and all three water-share sentences, because those two pages repeat them. (Counted after
  the edit, per span, not per file.) Nothing needed re-grounding, so no `source` value changes in
  this entry.

  ### The assertions that could not have caught this

  Five citation pins asserted only the HOST — `out.contains("whoi.edu")` — which is precisely the
  half of a URL a site reorganization leaves alone. They were green before this change and green
  after it, and would stay green through the next move. All five now pin the whole
  `"locator":"…","trust":"…"` pair.

  **8 of 8 mutants killed, plus a baseline control.** Reverting each file's locator to its stale form
  reddens that file's e2e test; unmutated, all eight are green.

  ### Not folded in, deliberately

  `nutrition/food-groups.adj` (23 rows, and **all 23** unmentioned by its envelope) is blocked on
  MyPlate: its citations cannot currently be read at the addresses they name, so the #14986
  conversion cannot be done honestly.

  A draft of this entry called it "the worst exemplar-span table in the stdlib". It is not, and
  finding out why exposed a bug in **my own audit instrument**: it dropped subject tokens shorter
  than four characters and then skipped any row left with no tokens, so every row whose subject is a
  short word — `dog`, `cat`, `fox`, `owl`, `pig` — could never be counted as unmentioned. It was
  reporting rows as grounded by spans containing no trace of them.

  Two more defects turned up in the same instrument once I started checking numbers instead of
  reading them: it **could not see single-line row blocks** at all (a converted row written
  `row (length, meter, "m") { source "…" }` puts the value mid-line, and the pattern was `^`-anchored),
  so `metrology/si-base-units.adj` — converted in #15073 — counted as 7/7 unmentioned; and one figure
  I published had been measured before a later conversion moved another 10 rows.

  Measured on today's tree: **1169 of 2105 rows (55.5%)** have an unmentioned subject, not 803
  (38.1%), and **68 tables** have every row unmentioned, not 39. **That is a lower bound** — the
  matcher counts a shared four-character stem as a mention (`pancreas` is "named" by *"pancreatic
  islets"*), leniency that exists to avoid false accusations and can only push the true number up.
  The largest are `language/word-families.adj` (31/31) and `biology/animal-babies.adj` (24/24), then
  `language/idiom-meaning.adj` and `food-groups` tied at 23/23. #15139 carries the full correction and
  a triage of the remaining 68 — 43 ordinary prose conversions, 21 blocked on the held weld question,
  4 on sources that cannot be read. The audit numbers in the table above are locator verdicts, from a
  different instrument, and are unaffected. The two DEAD citations
  (`optics/rainbow-colors.adj`, `physics/circuit-parts.adj`) each need a replacement source, and both
  are *also* exemplar-span tables. Probing so far is recorded on #15139 — `science.nasa.gov`'s visible
  light page is live but orders colors by wavelength, which is the reverse of that table's
  `red → 1 … violet → 7`, so it would make every ordinal an inference rather than a reading.

