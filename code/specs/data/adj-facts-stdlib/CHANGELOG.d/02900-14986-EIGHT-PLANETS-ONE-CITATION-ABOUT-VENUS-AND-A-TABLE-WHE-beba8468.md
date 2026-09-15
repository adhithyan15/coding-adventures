- **#14986: eight planets, one citation about Venus — and a table where the trust tier finally does
  visible work.** `astronomy/planets.adj` had one envelope, the sentence *"Venus is the second planet
  from the Sun"*. Ask for Jupiter's position and the citation was about Venus.

  Each row now carries the NASA hub page's own sentence for that planet, via RS-5e. No row restates
  `locator` — one page, inherited. The envelope becomes the **framing** span: *"Our solar system has
  eight planets: Mercury, Venus, Earth, Mars, Jupiter, Saturn, Uranus, and Neptune."*

  ### Seven rows are read. One is reasoned. The tier says which.

  Measured 2026-09-13 UTC with a raw fetch: each of the eight spans occurs **exactly once** in the
  raw HTML and once in the rendered text. Seven literally state their ordinal — *"the fifth planet
  from the Sun"*, *"the eighth and most distant planet"*.

  **Mercury's does not.** The page says *"Mercury is the planet nearest to the Sun"* — never "the
  first planet". Position 1 is read off "nearest" plus the ordering of a second span, *"The first
  four planets from the Sun are Mercury, Venus, Earth, and Mars."*, in which Mercury is named first.
  That is a reading, not a quotation, so **that row alone carries `trust inferred`** and both spans —
  the same treatment the tropic rows get in `geography/reference-lines.adj`.

  The distinction is pinned: flipping Mercury to `authoritative` reddens.

  An earlier draft of this entry called this "the first table in the series where the read/reasoned
  distinction shows up within one table". **That is false**, and falsified by the file this entry
  cites as its own precedent: `geography/reference-lines.adj` (#15056) already ships four rows
  inheriting `authoritative` beside two overriding to `inferred`. The claim was about my own work
  rather than about the page, and I did not check it.

  ### The envelope change is unreachable by construction, and was unpinned

  Review measured it: replacing the new framing span with *"Our solar system has nine planets:
  entirely fabricated."* left **every test green**. That is not a gap that an output-based pin can
  close — once all eight rows override `source`, the envelope can never reach an answer, so no CLI
  test can see its wording at all. Presenting "the envelope becomes the framing span" as a
  delivered, covered fix was overstated.

  What IS now pinned is the property: a test asserts that all eight rows answer AND that the
  framing span appears in none of them. If a future change let the envelope leak back into an
  answer, that reddens. The same unreachability holds for `metrology/si-base-units.adj` (#15073),
  where it was disclosed but not asserted.

  ### It propagates through a derived `rule`

  Measured, not assumed. `astronomy/planet-ordinal-position.adj` is a `rule` bridging this table to
  `mathematics/ordinal-numbers.adj`. After the change — summarised from the JSON, not captured
  verbatim, and each answer actually carries TWO citations (the NASA span at the row's tier, plus
  the ordinal-word source at `consensus`):

  ```
  planet_ordinal_position(jupiter, fifth)   trust=authoritative
      src: Jupiter is the fifth planet from the Sun, and the largest planet in our solar system.
  planet_ordinal_position(mercury, first)   trust=inferred   corroborations=1
      src: Mercury is the planet nearest to the Sun, and the smallest planet in our solar system.
  ```

  The derived answer carries the **row's own** span and the row's own tier — including `inferred`
  for the one reasoned row. So repairing a base table repairs every rule standing on it, without
  touching the rule. That is an argument for converting tables that have dependents first, and it
  was not obvious beforehand.

  ### Also corrected

  The header claimed every order was *"confirmed by its own NASA page"*. The spans are all from the
  **hub** page; the eight per-planet pages were never fetched and nothing rests on them. Now said
  plainly.

  ### Pins

  **9 mutants killed, 1 green by design, 1 restore control** — final state, after the coverage gap
  below was closed. (Two earlier drafts of this line were wrong in different ways: "9/9" conflated
  kills with a deliberate green, and a later "8 killed + 1 green control" was measured before four
  more rows were pinned.)

  Killed: Jupiter's span swapped for the old Venus envelope; Jupiter's, Neptune's and **Saturn's**
  blocks dropped; "fifth" changed to "sixth" in Jupiter's span; **Mercury's tier flipped**;
  Mercury's ordering `cites` dropped; **Mercury's `cites` locator repointed at Wikipedia**; the
  shared envelope locator dropped.

  **Green by design:** fabricating the envelope's framing span. See the unreachability section
  above — no output test can see its wording, and expecting otherwise was my error rather than a
  gap.

  **The coverage gap review found:** Saturn was not the single unpinned row. `venus`, `mars`,
  `saturn` and `uranus` were *all* entirely unpinned — a dropped block, a fabricated span, and even
  a **corrupted order value** each stayed green. All eight rows are pinned now, and cross-row
  isolation (a row-level mutant reddens only the test that queries it, verified for all eight)
  replaces the retired "queried by no test" control, which no longer has a subject.

  Controls on the span measurement: a nonsense span scores zero, and a near-miss of each of the
  SEVEN that carry an ordinal word (that word changed) scores zero. It is inapplicable to Mercury's
  span, which has no ordinal word — that being this table's whole point. The en dashes in the Earth sentence are the page's
  own U+2013 and survive byte-exact.

