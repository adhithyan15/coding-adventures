- **#14986: `language/noun-type.adj` — five noun types stop being warranted by the common-noun sentence, and two rows carry the whole sentence the header cut short.**
  The envelope was the common-noun row's own sentence, *"A common noun is the generic name of an item
  in a class or group."*, so it was the primary source of all six answers.

  ### What each row carries now

  Measured 2026-09-15 on Grammarly's noun article (HTTP 200; a nonsense path on the same host
  returns 404), with inline tags removed without a space and only ASCII whitespace collapsed. The
  nonsense page holds none of the sentences below.
  - **Every row takes its own sentence as a `source`.** Each of the six sentences names its type and
    its definition, and each occurs **exactly once**, inside a `<p>`.
  - **The envelope** is now the article's *"Nouns are everywhere in our writing."* It occurs once in a
    `<p>` and names no type and no definition.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  ### Header quotes that were not verbatim

  - **Two quotes stop short.** In a truth table whose column says "quote (verbatim)", the countable
    and uncountable quotes end before their sentence does and carry no period. On the page the first
    goes on with "(like the number of humans in the world)." and the second with a clause beginning
    "whether because they name". The table body never shipped either sentence. The two new rows carry
    the whole sentence; the header's quotes are kept and a note says they are cut.
  - **Emphasis added.** The three bundled-fact quotes (proper, singular, gerund) write a capitalized
    AND. The page writes "and". A note says so.
  - The title line said **three** noun types; the table has six.
  - The header named the article "Nouns: Definition and Examples", which the fetched page does not
    write (0 occurrences). Its `<h1>` is "What Is a Noun? Definition, Types, and Examples", and the
    header and the test's doc comment now use that.
  - The "WebFetch-verified before writing" note is marked as superseded by the raw-page measurement.

  ### Pins

  The test keeps every behaviour it shipped with: the direct and reverse recalls on both the original
  and the added rows, and the two abstentions. Its `contains("grammarly.com") && contains(trust)`
  check (#15209's shape) becomes the common row's whole contiguous citation. Added:
  - every type's answer is one answer carrying its own sentence, with no other type's sentence and not
    the envelope;
  - the countable and uncountable rows carry their whole sentences, not the header's cut forms;
  - a table-shape test.

  **9 of 9 mutants killed, two controls:**
  - the countable sentence cut to the header's quote;
  - the uncountable sentence cut to the header's quote;
  - the collective row warranted by the common sentence;
  - the abstract `source` demoted to `cites`;
  - the old envelope restored;
  - a table-level `cites` re-added;
  - a row restating a different trust tier;
  - a definition atom rebound;
  - a type named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
