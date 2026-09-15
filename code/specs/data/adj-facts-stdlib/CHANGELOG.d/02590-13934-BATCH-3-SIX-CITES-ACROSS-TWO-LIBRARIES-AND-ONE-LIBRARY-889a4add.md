- **#13934 batch 3: six `cites` across two libraries, and one library REMOVED from the batch after
  extraction.** `meteorology/precipitation-types` (+4 — snow, sleet, hail, freezing rain) and
  `biology/animal-habitat` (+2 — bactrian camel, giraffe).

  *** `anatomy/tooth-types` WAS PLANNED INTO THIS BATCH AND TAKEN OUT: its unencoded NCBI page does
  not support the row VALUES. *** The rows are `canines -> tearing` and `premolars`/`molars ->
  grinding`, and the page offers only "the canine teeth are sharp at the tip", "The back teeth just
  behind the canine teeth are called premolars", and "The molars look different: They have wide
  chewing surfaces". Sharp-at-the-tip is not TEARING; positional naming is not GRINDING;
  wide-chewing-surfaces is *close* to grinding and does not say it. "Close to" is exactly what the
  match-the-value rule exists to reject — the same rule that stopped `macronutrients` taking "high in
  calories" for "7 calories per gram".

  `precipitation-types`' SNOW row was expected to be sourceless — three header URLs for five rows. It
  is not: NOAA's glossary answers `word=SNOW` directly. Two of the four new locators (SNOW, SLEET) are
  therefore **not header-documented**; I found them. That is a slightly different activity from
  "encode what the header recorded" and is named rather than blurred. Its freezing-rain evidence, by
  contrast, was on the **already-encoded** page all along: `glossary.php?word=RAIN` returns every entry
  matching "rain", so the library was sitting on evidence it had already fetched.

  ON THE GLOSSARY HEADWORDS. Each quote is a definition BODY, and the locator is a query returning
  **eighty-five entries** — so the locator alone does not identify which entry a body came from. The
  headword matters. It became visible only because the extractor's `dt`/`dd` fix this session stopped
  `<dt><b>Hail</b></dt><dd>Showery...` fusing into "HailShowery" — but there is no contiguous RENDERED
  span joining a headword to its body, so joining them would CONSTRUCT text the page does not display.
  The bodies are quoted as they stand and the ambiguity is recorded.

  *** A FIFTH EXTRACTOR LIMITATION, AND THE MOST MISLEADING ONE. *** The giraffe page returns HTTP
  308, which Python 3.10's urllib does not follow. It raised `HTTPError` and looked **identical to a
  dead source** until the exception text was read. Without following it, `giraffe -> grassland` would
  have been recorded as a FALSE BLOCKER on a page that is perfectly fine — and the row's evidence is
  right there: "Grassland, for example, is the habitat of the giraffe...".

  NOAA's glossary declares UTF-8 and serves a mojibake `Â½` in the SLEET entry (UTF-8 bytes for ½ read
  as Latin-1). The corruption is in the SOURCE, not the decoder; every span quoted here excludes it.

  `language/onset-rime` is DEFERRED, not refused — its evidence is simply not yet extracted.

  Both `.query.adj` companions parse and run. 533 test binaries / 1592 tests green, clippy -D warnings
  clean. Every sentence verified verbatim against a fresh raw-HTML normalisation.

