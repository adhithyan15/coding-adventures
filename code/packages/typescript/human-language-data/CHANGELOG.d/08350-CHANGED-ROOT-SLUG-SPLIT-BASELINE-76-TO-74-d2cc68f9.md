### Changed — root-slug split baseline 76 to 74

An etymology cleanup across the Russian, Latin, Spanish, Portuguese, Malayalam, Tamil, Bengali, Hindi, Marathi, Marwadi, Punjabi and Sanskrit tracks gave several etymons one root slug and split one slug that covered two Latin verbs. Two changes resolve known splits, so the pin falls twice.

- **`core/root-slug-split-baseline.json` 76 → 74 entries.** Both entries were removed by hand, matching what `generate:root-slug-splits` would prune.
  - `ne|` (`ne` against `pie-ne`): RU-C01-net and RU-C16-ni-ni, the only users of bare `ne`, now use `pie-ne`, the slug HI-C07-nahin and GE-C18-nein give the same PIE *\*ne*.
  - `ne-negative|` (`ne-negative` against `pie-ne-negative`): LA-C01-ita-non and PA-C31-na, one user each, both named PIE *\*ne* and now use `pie-ne` too.
- `tests/root-slug-splits.test.ts` pins 74, with a comment for each step.
- No new split entry was added. The other slug changes merge spellings the split check does not pair (`sanskrit-raja` → `sanskrit-rajan`, `indo-aryan-yotayati` → `indo-aryan-yutati`, `shoc-grieve` → `sanskrit-shocyate`, `ossum-latin` → `os-latin`, `tamil-mani-bell` → `dravidian-mani-bell`), move ES-C309-pariente off `parere` (*pāreō*) onto `parens-latin` and `parere-bear-latin` (*pariō*), give ES-C408-pastilla a new `pastillus-latin` in place of a false `pasta-spanish` join, add `sanskrit-shuc` to HI-C34-sochna, or rename Malayalam slugs to match their text (`dravidian-accan-father` → `prakrit-ajja`; `sanskrit-mani-gem-bell` → `dravidian-mani-bell`).
