---
category: CI & GitHub Actions
---

# Run check:root-slug-splits before pushing new root slugs

The Bengali B1 vocabulary minted four root slugs (`english-bonus`, `english-senior`,
`sanskrit-kshama`, `arabic-raqm`) for etymons the corpus already files as `bonus`,
`senior`, `kshama-sanskrit` and `raqm-arabic`. Every local vitest selection passed, but
the "Build all human-language books" job runs `npm run check:root-slug-splits`, which
refuses a new spelling of a known etymon (a slug is the cousins join key, so two
spellings silently split a cousin panel).

Fix: reuse the corpus spelling in the lessons and the generator spec, then regenerate
books, narration and modality (the cousin panels of the affected chapters change).

Next time: run `npm run check:root-slug-splits` in
`code/packages/typescript/human-language-data` after generating any lessons with new
roots, and before minting `<lang>-<word>`, grep the corpus `roots:` lines for the bare
word and its other spellings.
