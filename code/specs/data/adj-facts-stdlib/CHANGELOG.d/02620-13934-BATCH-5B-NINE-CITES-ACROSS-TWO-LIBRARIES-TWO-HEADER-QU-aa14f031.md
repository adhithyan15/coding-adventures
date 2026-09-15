- **#13934 batch 5b: nine `cites` across two libraries, two header quotes corrected, and one
  pre-existing assertion repaired.** `money/us-coins` (+6 — penny, nickel, dime, quarter,
  half_dollar, dollar) and `chemistry/element-groups` (+3 — alkaline_earth_metal, halogen,
  noble_gas).

  *** THE HALOGEN HEADER QUOTE TRUNCATED A CAVEAT ABOUT ITS OWN ROW. *** It read "…consisting of
  six chemically related elements**:** fluorine (F), … tennessine (Ts)." The page reads
  "…elements**,** fluorine (F), … tennessine (Ts), **though some authors would exclude tennessine
  as its chemistry is unknown and is theoretically expected to be more like that of gallium.**"

  A colon replaced a comma, and the quote stopped immediately before the clause saying some authors
  exclude tennessine — while the table ships `row (tennessine, halogen)`. That is the same defect as
  `brain-parts`' hippocampus quote: truncating right before the qualification that bears on the
  claim. The sentence is now cited whole. **Where the page qualifies its own claim, the
  qualification is part of the evidence, not noise to trim.** `noble_gas`' quote elided its
  parenthetical with "…" — a constructed span — and is likewise cited whole; the `[1]` footnote
  marker is real rendered text and stays.

  *** A PRE-EXISTING TEST ASSERTED A SUBSTRING WHERE IT MEANT A STRUCTURE, AND ENCODING REAL
  EVIDENCE TRIPPED IT. *** `assert!(!out.contains("oganesson"))` was standing in for "oganesson is
  not a row". Encoding the noble-gas sentence puts "oganesson (Og)" into the output as quoted
  *evidence*; no row was added and the reverse recall still returns exactly six gases, but the
  assertion failed anyway.

  The sentence that tripped it is the same one the test's own comment cites as its justification
  ("its source sentence hedges it 'in some cases'") — the check forbade the output from carrying the
  evidence for its own reasoning. Replaced with structural assertions: no `element_group_family(
  oganesson, noble_gas)` governing term and no `"E":"oganesson"` binding. A sweep found six
  `!out.contains("…")` assertions in the suite and **oganesson was the only single bare word**; the
  rest are atom-shaped (`cave_ceiling`, `search_limit_exceeded`), a form prose never produces. A
  lone case, not the tip of a class.

  ON `us-coins`: **four of the six are present-tense definitions and two are not.** `nickel`, `dime`,
  `half_dollar` and `dollar` each get "The nickel is the United States' five-cent coin". `penny` and
  `quarter` do not, because their pages are written historically — penny's sentence is about the coin
  ceasing to circulate and never uses the word *penny*, quarter's is about an 1804 marking. Both
  still state their row's value and both locators resolve to exactly one coin, so both are cited, but
  they are oblique where the other four are direct. `money/coin-penny-discontinued.adj` already
  carries the penny sentence as its own envelope — not a duplication, but it is why that sentence
  reads as being about discontinuation, and grepping the stdlib first is what surfaced it.

  `transition_metal` is LEFT UNCITED: its header sentence is not on the live page under any extractor
  fix, and the nearest candidate ("Metallic iron and the alloy alnico are examples of ferromagnetic
  materials involving transition metals") names only iron, indirectly, for a row set of
  iron/cobalt/nickel. Category, not value.

  A NARROWED BLOCKER, twice over. The "Wikipedia evidence + `cites` has no trust field" problem is
  **not** a class-level block. `element-groups`, `muscle-groups` and `animal-homes` all declare
  `trust consensus` — the right tier for an encyclopedia — so nothing is overstated. Only
  `kidney-parts` declares `trust authoritative`, and the gap is **directional**: a cite *weaker* than
  its envelope overstates the evidence, while a cite *stronger* than its envelope merely understates
  it and is harmless. That unblocks `muscle-groups` and `animal-homes` for a later batch.

  Each library gets a prefix pin and a **whole-list** pin, all four cut from real CLI output and read
  from a saved file rather than retyped. Five directional mutations pass, including the two that
  justify the whole-list form: a pure **reorder** of two middle entries reddens it while the prefix
  pin stays green, and **dropping the tennessine caveat** — the exact truncation the shipped header
  had made — reddens it too.

  Both `.query.adj` companions parse, run, and abstain correctly. 533 test binaries / 1604 tests
  green, clippy `-D warnings` clean.

