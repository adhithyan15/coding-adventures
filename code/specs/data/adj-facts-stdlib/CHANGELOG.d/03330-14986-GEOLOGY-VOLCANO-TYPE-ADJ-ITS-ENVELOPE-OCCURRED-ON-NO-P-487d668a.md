- **#14986: `geology/volcano-type.adj` — its envelope occurred on no page, and the cinder and composite types stop being warranted by the shield sentence.**
  The envelope was the shield row's own sentence, so it was the primary source of all three answers. As
  shipped it was also a string the page never writes: *"Shield volcanoes are built almost entirely of
  fluid lava flows."* with plain spaces, where the page writes U+00A0 on both sides of "lava". In that
  form it occurs zero times.

  ### What each row carries now

  Measured 2026-09-15 on USGS's "About Volcanoes" page (HTTP 200; a nonsense path on the same host
  returns 404), with inline tags removed without a space and only ASCII whitespace collapsed. The
  nonsense page holds none of the sentences below.
  - **Every row takes its own sentence as a `source`, in the page's characters.**
    - Cinder cone: "Cinder" U+00A0 "cones are the simplest type of volcano." The plain-space form occurs
      zero times.
    - Shield volcano: the shield sentence with U+00A0 before and after "lava".
    - Composite volcano: the composite sentence, with its U+2014, as the header already quoted it.
  - **The envelope** is now the page's *"There are about 1,350 potentially active volcanoes worldwide,
    not counting the volcanoes under the oceans."* It names no type.
  - Each of the three sentences and the envelope occurs **exactly once**, inside a `<p>`, both with
    script and style data set aside and over the whole file.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  ### Header corrections

  - **Two truth-table quotes aren't the page's characters.** A comment can't show U+00A0, so the cinder
    and shield quotes keep plain spaces, and a note says that in that form each occurs zero times.
  - **The lava-dome disclaimer** was quoted with single quotes around volcano type. The page writes
    double quotes, and the single-quote form occurs zero times. The quote now uses double quotes.
  - **The "WebFetch-verified before writing (twice ...)" note** is marked as superseded by the raw-page
    measurement.

  ### Pins

  The test keeps every behaviour it shipped with: the direct recall of the shield volcano, the reverse
  recall of the cinder cone, and the abstention on lava_dome. Two pins change:
  - **The direct recall's "whole citation" pin** was the plain-space shield sentence, the string the
    page never writes, so the test passed against a citation no reader could find. It now pins one
    answer whose citations array holds exactly the shield sentence in the page's characters, and
    asserts the plain-space form is absent.
  - **The host-plus-trust needle** (#15209's shape) is gone.

  Added:
  - every type's answer is one answer whose citations array holds exactly its own sentence, with no
    other type's sentence and not the envelope;
  - the rows carry the page's U+00A0 (one in the cinder sentence, two in the shield), no plain-space
    form is left in the table, and no single-quoted lava-dome quote is left in the header;
  - a table-shape test.

  **10 of 10 mutants killed, two controls:**
  - the shield sentence losing its no-break spaces (the shipped defect);
  - the cinder sentence losing its no-break space;
  - the composite sentence's U+2014 replaced by a hyphen;
  - the cinder row warranted by the shield sentence;
  - the composite `source` demoted to `cites`;
  - the old plain-space envelope restored;
  - a table-level `cites` added;
  - a row restating a different trust tier;
  - a description atom rebound;
  - a type named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
