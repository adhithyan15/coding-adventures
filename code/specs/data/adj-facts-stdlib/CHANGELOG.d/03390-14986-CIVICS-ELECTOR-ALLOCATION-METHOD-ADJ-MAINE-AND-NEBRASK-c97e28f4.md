- **#14986: `civics/elector-allocation-method.adj` — Maine and Nebraska stop being cited to the sentence about the other 48 states.**
  The envelope was the group row's own sentence, *"In 48 states and Washington, D.C., the winner gets
  all the electoral votes for that state."*, so it was the primary source of all three answers. The
  Maine and Nebraska method was read across a table-level `cites`. The shipped test pinned exactly that:
  Maine's answer was cited to the sentence about the *other* 48 states.

  ### What each row carries now

  Measured 2026-09-15 on USA.gov's "Electoral College" page (HTTP 200; a nonsense path on the same host
  returns 404), with inline tags removed without a space and only ASCII whitespace collapsed. The
  nonsense page holds none of the sentences below.
  - **`forty_eight_states_and_dc`** takes the 48-states sentence.
  - **`maine` and `nebraska`** each take *"Maine and Nebraska assign their electors using a proportional
    system."*, which names both states and the method.
  - **The envelope** is now the page's *"The Electoral College decides who will be elected president
    and vice president of the U.S."* It names no jurisdiction and no method.
  - **Counts:** each row sentence occurs **exactly once**, both with script and style data set aside and
    over the whole file. The two sentences sit consecutively in one `<p>` inside a list item, which
    matches the header's "two consecutive sentences". The envelope occurs exactly once, inside a `<p>`.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  ### Pins

  The test keeps every behaviour it shipped with: Maine's direct recall, the two-state reverse recall,
  the group rule, the composition with `electoral-college-count.adj`, and the two abstentions
  (California, and the unexplained mechanism). Three pins change:
  - **Maine's direct recall** pinned the 48-states sentence as its `"source"`. It now pins one answer
    whose citations array holds exactly the Maine and Nebraska sentence, and asserts the 48-states
    sentence is absent.
  - **The reverse recall** now also pins two answers, each whose citations array holds exactly that
    sentence.
  - **The group rule** pinned a fragment of its sentence. It now pins one answer whose citations array
    holds exactly the whole 48-states sentence, with the exception sentence absent.

  The host-plus-trust needle (#15209's shape) and the fragment needles are gone. Added:
  - every jurisdiction's answer is one answer whose citations array holds exactly its own sentence,
    with no other row's sentence and not the envelope;
  - a table-shape test.

  **9 of 9 mutants killed, two controls:**
  - Maine warranted by the 48-states sentence;
  - Nebraska's `source` demoted to `cites`;
  - Maine reverted to a bare row that inherits the envelope;
  - the group row warranted by the Maine and Nebraska sentence;
  - the old envelope restored;
  - the table-level `cites` re-added;
  - a row restating a different trust tier;
  - a method atom rebound;
  - a method named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
