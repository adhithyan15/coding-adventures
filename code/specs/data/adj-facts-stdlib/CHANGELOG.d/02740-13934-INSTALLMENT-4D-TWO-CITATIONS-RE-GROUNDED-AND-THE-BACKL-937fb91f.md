- **#13934 installment 4d: two citations re-grounded — and the backlog's "easy" band was mostly
  held questions in disguise.** 2 value lines, 2 `.adj` files, `geography/landform-secondary-feature`
  and `anatomy/muscle-groups`. Two mutations redden. Contiguity **72 → 70**, verified by
  diffing the non-contiguous SETS: the two that left are the two repaired, none entered.

  **A FABRICATED FULL STOP.** `landform-secondary-feature:81` ended the canyon definition with a
  period. The USGS Feature Type Thesaurus has none — a bracketed source citation follows the
  definition on the page. One character, existing nowhere on the page, in a field whose entire
  contract is that its bytes are on that page. It reads like punctuation hygiene, which is why no
  screen built for quotes, elisions or dashes could see it.

  **A SILENTLY ELIDED PARENTHETICAL.** `muscle-groups:149` dropped the page's Latin gloss —
  `(Latin: musculus biceps brachii, "two-headed muscle of the arm")` — with no marker. What makes it
  a slip rather than a policy is the same table: **seven of its eight `cites` values keep theirs**
  (the gluteus maximus sentence has no parenthetical to keep), including the triceps one three lines
  below, which is the identical construction on the identical kind of page. One of eight was tidied.

  **Review then found the same fabricated period in a SIBLING header**, `geography/landforms.adj`,
  quoting the same definition from the same page — after the repair, the only surviving copy in the
  tree, and it falsified a live claim in the file just repaired: `landform-secondary-feature`'s
  header says it "reproduces, **byte-for-byte**, the SAME five spans already quoted inside
  `landforms.adj`'s own header". One byte apart, that was false. Fixed in the same edit.

  The desync screen reports 0 and is right to: it compares a header against the values in ITS OWN
  FILE. This one is CROSS-FILE, between two headers quoting one page, and no instrument here looks
  for it. Recorded on #14111 as a third axis rather than left implied.

  ### The held set was never a census, and the triage was ranking held questions as easy work

  The held set in use — 16 CROSS-BLOCK + 7 LATEX-MATH — came from a classifier run **once**, over the
  pre-4a list of 84, and has been carried forward for three installments as though it were a count.
  Reading three candidates instead of trusting their scores found three more that belong in it, and
  every one of them was ranked *easier* than the two repaired here:

  | value | triage | what the page actually says |
  |---|---|---|
  | `physics/wave-properties:99` | 88% | `The wavelength λ λ is…` — MathJax emits the symbol twice; a reader sees one |
  | `geometry/shapes:39` | 86% | MathWorld renders the row as two cells, `3` and `triangle (trigon)`; the shipped `\|` separator is on no page under any joining |
  | `geometry/quadrilateral-types:148` | 70% | `…lengths and , and with…` — the variables `a` and `b` are images the extractor cannot see |

  All three are already faithful spans of the page **as rendered**. Repairing them would answer a
  held question by shipping, and an 88% score is exactly what makes that look like a chore.

  So the held set was re-derived **structurally** — words-in-order-spanning-cells, adjacent duplicate
  token folding, and a difflib alignment where the value's extra tokens must be short *and* the page
  must have a real gap there. No lists of Greek letters or `|` separators: character-keying is the
  mistake this effort has made seven times.

  **The two classifications disagree in BOTH directions, so neither is published as the number.**
  The structural census finds cross-block values the old one missed (it keyed on the literal `" | "`,
  blind to a row whose cells the extractor joins with spaces); the old one holds values the census
  calls actionable, because MathJax renders `\(90^{\circ}\)` as `90°` and the census does not model
  a rendering the extractor performs. Reconciled: **33 agreed-actionable, 23 disputed, 16
  agreed-held**. Only the 33 were eligible — measured on the pre-repair 72, so **31 remain** after
  this installment — and every disputed value is named on #14111 rather than averaged away.

  ### Locator liveness: a question no screen here had asked

  Every screen so far asks whether a value matches its page. None asked **whether there is still a
  page**. `physics/circuit-parts:112` cites a PDF at `k12maker.mit.edu` that **404s** — the host is
  alive, the document is gone. It had sat in the contiguity screen's `fetch failed` bucket across
  three sweeps, correctly bucketed as UNCHECKED rather than clean, and no installment went and
  looked. The bucket did its job; I did not.

  A sweep of all **314 unique locators** (they carry all 568 shipped values; the 562 in contiguity
  scope use 309 of them) says **1 dead, 8 redirected to a different path, 0 unreachable** — plus one
  more that redirects only by gaining a trailing slash, which the screen folds into ALIVE. Nine
  locators move if that fold is not applied, and saying which rule is in force is the difference
  between a measurement and a number — but the sweep's own load corrupted its first answer, which was *3 dead*. Two NIH
  `ods.od.nih.gov` pages answer **200** when probed alone; the 3-attempt backoff was tuned against a
  single value, not against 314 back-to-back. Re-probing every non-ALIVE verdict one at a time, with
  a live-page control in the same run, is what turned 3 into 1.

  **A THIRD HELD QUESTION, found by that sweep.** All three cited `myplate.gov` pages now redirect to
  a stub site root — byte-identical 27,959-byte bodies — and USDA has superseded MyPlate with
  `realfood.gov`. Those three locators back **5 of the 70**, and they can never go contiguous: the
  screen fetches the canonical URL and finds a stub. This is not an oversight to repair.
  `food-groups.adj`'s own header records that a prior installment met this and chose deliberately —
  cite the canonical URL, verify against a National Archives capture. Whether `locator` should name
  the canonical address of a document or an address where its bytes can still be fetched is a
  question of the same kind as the other two, and it is not mine to settle by shipping.

  ### Instrument defects, all found by controls rather than by reading the code

  - The liveness screen's first version reported **everything unreachable, Wikipedia included** — it
    passed `-o /dev/null` to a Windows curl through `subprocess` with no shell. A screen that says
    "unreachable" for every input looks exactly like a network outage.
  - Its second called a nonexistent host **DEAD**: curl writes `-w` output even when the transport
    fails, stamping `http_code 000`, and reading that as a server answer collapses "the server said
    no" into "I never reached a server" — the two-buckets-instead-of-four mistake, committed inside
    the file whose docstring warns about it.
  - Blind duplicate-token folding ate **"had had enough"**. A screen that rewrites the page until the
    value fits is not a screen.
  - The dropped-glyph test tried twice to *guess* which tokens were variables; `a`, `is`, `of` are
    all short, so it blew its own cap on ordinary English and silently bailed.
  - The census classified into five buckets and **printed four** — 24+1+2+41 came to 68 of 72 and the
    gap went unremarked. It now refuses to report a bucket it cannot name.
  - `triage84.py` reads the pre-4a sweep and wraps its fetch in `except Exception: continue`; with
    two rate-limiting hosts in play that silently drops real candidates. Replaced.

  ### A test-harness collision that would have passed while testing nothing

  The new canyon test used `scratch("reground")` — a tag installment 4b had already used **in the same
  file**. `scratch()` keys on tag + process id, and cargo runs a binary's tests as parallel THREADS of
  one process, so both tests shared a directory and each wrote `case.adj` into it: the canyon test
  silently executed 4b's *plateau* query. It failed only because the pin did not match the wrong
  answer; with two similar queries it would have passed while testing nothing — the same shape as a
  cargo filter matching zero tests and printing `ok`. A standing screen now checks every binary:
  **527 scanned, 0 duplicate tags**, so the one I created was the only instance.

  Three **pre-existing** `muscle-groups` pins went red on the repair, correctly: that library is a
  single `table`, so its one envelope rides on every answer and three sibling pins quote it verbatim.
  Updated to the repaired text, never loosened.

  *Contiguity numbers are measured over* `source`/`cites` values of at least 20 characters with a
  resolvable locator (562 of 568), compared after stripping tags with **no separator inserted at tag
  boundaries**, unescaping entities, NBSP→space and collapsing whitespace.

  *The `fetch failed` bucket held **2** in both sweeps and swapped a member — `biology/vitamins:122`
  out, `biology/vitamin-deficiency-symptom:84` in, both NIH rate-limit artifacts that re-screen clean
  when probed alone. Equal totals hiding a swap is exactly why these deltas are taken by diffing sets.
  The unjudgeable set (36) is identical member-for-member.*

  `cargo test -p adj-lang-cli`: the two touched binaries green (7 and 5 tests); clippy `-D warnings`
  clean; both `adj_stdlib_*` gates exit 0; the header/data desync screen reports 0. A full-workspace
  run is still impossible on this box — ten packages do not build on Windows — so that claim stays
  with CI.

