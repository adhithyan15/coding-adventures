### Changed — `maxLinearisableTableColumns` moves from 0 to 3

- The knob shipped at **0** in the modality slice on purpose: no lineariser existed,
  and claiming a table was speakable would have claimed a capability nothing
  implemented. The lineariser now exists, so the default is its measured value, **3**,
  and it is authored in `core/chapter-policy.json` (validated on load: an integer from
  0 through 16) rather than living only as a constant.
- Three, and not four, because that is where a table stops being a list of labelled
  facts a listener can hold — *"Language: Telugu. Hello: namaskāram. Source:
  Sanskrit."* — and starts being a grid whose meaning lives in the comparison *across*
  rows. The corpus's own four-column tables prove the point: `| | numeral | word | said |`
  has an unlabelled first column that means something only because of where it sits on
  the page. Measured over the 340 table-bearing lesson files: 99 are 2 columns wide,
  173 are 3, 60 are 4, and 8 are 5 or more.
- At width 3 the lineariser reads **371 of the corpus's 442 tables (84%)**, covering
  272 of the 340 table-bearing files. The corpus moves from **694 drivable lessons
  (63%) to 925 (84%)**. Of the 120 that still need eyes, 65 carry a wide table, 61
  point at the page in prose, 7 have a `script` block, and **52 need eyes for a wide
  table and nothing else** — HL08's table-remediation burn-down list, now measured.
- `modality.ts`'s `wide-table` rule no longer means "wider than N". It means *"the
  narration lineariser refuses it"*, which is strictly larger: a three-column table
  inside the limit is still unspeakable when its rows are ragged. Asking the exporter's
  own judgement is the only way `voice` can be a promise the export is able to keep.
- `report-cli.ts` reads the same policy width, so the published drivable percentages
  and the committed narration export can never be computed at different settings.
- `tableRowColumns` now delegates its cell splitting to `speech.ts`, so the count a
  lesson is judged on is produced by the same scan the narration is built from.

