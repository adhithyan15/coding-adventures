- **#14986: `physics/light-behaviors.adj` — four light behaviors stop being warranted by the reflection sentence, and a header paraphrase stops wearing quotation marks.**
  The envelope was the reflection row's own sentence, *"Reflection is when incident light (incoming
  light) hits an object and bounces off."*, so it was the primary source of all five answers. The header
  said so on purpose, "An ADJ `table` carries ONE provenance envelope", and only listed the other rows'
  spans.

  ### What each row carries now

  Measured 2026-09-15 on NASA Science's "Wave Behaviors" page (HTTP 200; a nonsense path on the same
  host returns 404), with inline tags removed without a space and only ASCII whitespace collapsed. The
  nonsense page holds none of the sentences below.
  - **Every row takes the sentence its header already quoted** as a `source`. Each names its behavior
    and the effect.
  - **The envelope** is now the page's *"Light waves across the electromagnetic spectrum behave in
    similar ways."* It names no behavior and no effect.
  - **Counts:**
    - Each of the five row sentences occurs **exactly once** in the whole file, inside a `<p>`.
    - The envelope occurs once in the page body, inside a `<p>`. The page's `<head>` metadata repeats
      it once as the page description.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  ### Header corrections

  - **A paraphrase in quotation marks.** The opening summary ("Reflection is when light bounces off;
    refraction is when it changes direction ...") sat in quotation marks. The page never writes it, and
    it occurs zero times. The marks are removed and a note says it's a paraphrase.
  - **The table's citation.** "Carrying the table's citation" becomes "carrying its row's citation".
  - **The one-envelope paragraph** is replaced by the measurement.
  - **"The same page's opening sentence".** The header said the set of behaviors is named there. The
    sentence it quotes ("When a light wave encounters an object, they are either transmitted,
    reflected, ...") occurs once, but it is the second sentence of its paragraph, after the new
    envelope. The header now says so.

  ### Pins

  The test keeps every behaviour it shipped with: the forward recalls of reflection, refraction and
  diffraction, the reverse recall to reflection, and the abstention on gravity. Its
  `contains("science.nasa.gov") && contains(trust)` check (#15209's shape) becomes four answers, each
  whose citations array holds exactly its own row's sentence. Added:
  - every effect's answer is one answer whose citations array holds exactly its own sentence, with no
    other behavior's sentence and not the envelope;
  - a table-shape test.

  **9 of 9 mutants killed, two controls:**
  - the scattering row warranted by the reflection sentence (both say "bounces off");
  - the refraction sentence cut before "as they pass";
  - the absorption `source` demoted to `cites`;
  - the diffraction row reverted to a bare row that inherits the envelope;
  - the old envelope restored;
  - a table-level `cites` added;
  - a row restating a different trust tier;
  - an effect atom rebound;
  - a behavior named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
