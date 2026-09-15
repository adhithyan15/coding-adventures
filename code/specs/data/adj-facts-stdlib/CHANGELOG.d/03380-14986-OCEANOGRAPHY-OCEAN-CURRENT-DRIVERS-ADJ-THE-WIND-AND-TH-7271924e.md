- **#14986: `oceanography/ocean-current-drivers.adj` — the wind and thermohaline drivers stop being warranted by the tides sentence.**
  The envelope was the tidal row's own sentence, *"Tides create a current in the oceans, which are
  strongest near the shore, and in bays and estuaries along the coast."*, so it was the primary source
  of all three answers. The header said the table's `source` "carries the first row's (tidal_currents)
  quote" and documented the other two in the header.

  ### What each row carries now

  Measured 2026-09-15 on NOAA National Ocean Service's "What is a current?" page (HTTP 200), with inline
  tags removed without a space and only ASCII whitespace collapsed. A nonsense path on the same host
  also returns 200, with a soft-404 page titled "Page Not Found: Error 404", so the control is its
  content: it holds none of the spans below.
  - **Tidal and wind** each take the sentence the header already quoted for them. The wind sentence
    keeps the page's U+0027 in "ocean's".
  - **Thermohaline** takes the page's numbered label with its sentence: *"3. Thermohaline circulation.
    This is a process driven by density differences in water due to temperature (thermo) and salinity
    (haline) variations in different parts of the ocean."* "This is a process" alone names no current
    type.
  - **The envelope** is now the page's *"Oceanic currents describe the movement of water from one
    location to another."* It names no current type and no driver.
  - Each of the three spans and the envelope occurs **exactly once**, inside a `<p>`, both with script
    and style data set aside and over the whole file.
  - The header's claim that the page numbers exactly these three mechanisms holds: the rendered
    paragraphs open "1.", "2." and "3.", and none opens "4.".
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  The provenance note's "quoted verbatim", "carries the first row's quote" and "WebFetch-verified" wording
  is replaced by the measurement. Three other header lines are corrected:
  - **The truth table's thermohaline quote** now includes the "3. Thermohaline circulation." label, so
    the "quote (verbatim)" column matches the row.
  - **"Drive the deep, slow global circulation"** becomes "drive thermohaline circulation". The page says
    those currents occur "at both deep and shallow ocean levels".
  - **The Gulf Stream quote** gains an ellipsis. The page's sentence goes on with ", much further south."

  ### Pins

  The test keeps every behaviour it shipped with: the direct recall of wind, the reverse recall to tidal
  currents, and the abstention on the Gulf Stream. Its `contains("oceanservice.noaa.gov") &&
  contains(trust)` check (#15209's shape) becomes one answer whose citations array holds exactly the
  wind sentence. Added:
  - every driver's answer is one answer whose citations array holds exactly its own span, with no other
    type's span and not the envelope;
  - the thermohaline row carries its label, and no row carries "This is a process" alone;
  - a table-shape test.

  **10 of 10 mutants killed, two controls:**
  - the thermohaline row cut to "This is a process" alone;
  - the wind sentence's apostrophe curled;
  - the wind row warranted by the tides sentence;
  - the thermohaline `source` demoted to `cites`;
  - the tidal row reverted to a bare row that inherits the envelope;
  - the old envelope restored;
  - a table-level `cites` added;
  - a row restating a different trust tier;
  - a driver atom rebound;
  - a driver named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
