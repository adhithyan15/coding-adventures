- **#14986: `astronomy/solar-eclipse-type.adj` — annular and partial stop being warranted by the total-eclipse sentence.**
  The envelope was the `total_solar_eclipse` span, so asking what an annular eclipse is returned
  `moon_at_or_near_its_farthest_point_from_earth` evidenced by *"A total solar eclipse happens…,
  completely blocking the face of the Sun."* — a sentence about a different eclipse type. The annular
  and partial sentences were header prose that reached no answer.

  ### What each row carries now

  Measured 2026-09-16 UTC by this table's own fetch of the NASA "Types of Solar Eclipses" page
  (HTTP 200, 276 381 bytes), with inline tags removed without a space and only ASCII whitespace
  collapsed.
  - **Each row takes the NASA sentence defining its own type.** Each occurs **exactly once**, inside
    one `<p>`, scripts set aside and over the whole file, with no `<head>` copy.
  - **The envelope** is the page's sentence defining a solar eclipse as such, which names none of
    `total`, `annular`, `partial` or `hybrid` as a whole word. Once, in one `<p>`, no `<head>` copy.
  - **The nonsense-path control is not a stub.** A nonsense path on the same host answers 404 with a
    full styled **223 301-byte** error page. So the control here is that the spans are absent from a
    LARGE page, not that the page is empty.

  ### The envelope's apostrophe is U+2019

  The page writes "Sun’s" with U+2019. Counted on the fetched page: the envelope sentence with that
  apostrophe occurs **once**, and the same sentence with an ASCII apostrophe occurs **zero** times —
  so shipping the ASCII spelling would cite a sentence the source does not contain, the defect #15324
  records for `soil-texture-class` and the one `volcano-type` and `metamorphism-cause` both shipped
  before their conversions. The envelope is extracted from the fetched page rather than retyped, and
  `the_envelope_keeps_the_pages_own_apostrophe` pins both directions.

  (The count is about the quoted span. This header's own prose uses ASCII apostrophes throughout.)

  ### Pins

  - **Kept:** the forward bind, the reverse bind, and the `hybrid_solar_eclipse` abstention — a real
    type the same page covers, whose own explanation takes two sentences rather than one.
  - **Re-pointed:** the direct test already pinned the whole `"source"` sentence (#13916/#13918), but
    that sentence was the envelope, so it rode on every answer including the annular and partial ones.
    It is now the total row's own whole citations array, closing on both the corroborations `]` and the
    citations `]` (#14735).
  - **Inverted:** `contains("nasa.gov") && contains("\"trust\":\"authoritative\"")` — satisfied by any
    NASA citation, constraining no sentence text.
  - **Added:** a per-type check; the apostrophe test above; and a table-shape test (three row sources,
    keyword-anchored `cites` absence, **exactly one** `locator` and **exactly one** `trust` line, and a
    whole-word type-name check on the envelope).

  ### The first mutation run scored 8 of 10, and both misfires were mine

  **10 of 10 mutants are now killed by their named assertion**, both controls green, both files
  byte-identical afterwards. The two that failed first were harness faults, not weak tests:

  - **An anchor built from the wrong spelling matches zero times.** The mutant that makes the envelope
    name a type must also move the test's `ENVELOPE` const, or the verbatim-envelope arm fires first.
    My co-edit anchored on the const with a literal U+2019 — but the test file spells it as the escape
    `\u{2019}`. Reading line 137 settled it; the anchor now uses the file's own spelling.
  - **A derived value cannot be exercised by mutating its source.** `ascii_variant` is computed from
    `ENVELOPE` at runtime, so swapping the `.adj` apostrophe makes the FIRST arm fail on the missing
    curly form and the second arm is never reached. Exercising `!body.contains(&ascii_variant)`
    required the shipped file to hold BOTH spellings: keep the curly envelope, and plant an ASCII copy
    as a row source.

  It's a local scratch harness, so that count can't be reproduced from the repo.

  ### The query example, reported as it actually runs

  2 answers and 1 abstention; the envelope's wording occurs **zero** times in the output; 2 citations
  arrays and **no** non-empty corroborations. The example queries the total type directly and the
  annular type in reverse, so the **partial** span occurs **zero** times in its output — that row is
  exercised by the test suite, not by this example.

  The query file needed no change: it describes only importing the table and asking binding queries,
  with no claim about what the citation carries. The README row describes the axis, the abstention and
  the picking method, so it needs no change either.

  ### One stale verification note, superseded rather than deleted

  The header's provenance paragraph still read "WebFetch-verified before writing (twice)" — a claim
  about *how* the spans were checked, now sitting directly above a raw-fetch measurement that checked
  them a different way. The sibling `lunar-eclipse-type.adj` already had this exact situation and
  resolved it in place, so this file now uses the same form: the original note is quoted and the
  raw-page measurement is said to supersede it, rather than the note being silently dropped. Found by
  the security review, which read the shipped header rather than only the diff.
