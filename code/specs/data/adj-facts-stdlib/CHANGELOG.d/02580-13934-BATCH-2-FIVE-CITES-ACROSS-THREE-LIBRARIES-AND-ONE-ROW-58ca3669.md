- **#13934 batch 2: five `cites` across three libraries, and ONE ROW DELIBERATELY LEFT UNCITED.**
  `geometry/triangle-types` (+2, isosceles and scalene each from their own MathWorld page),
  `biology/macronutrients` (+1, alcohol), `physics/lens-types` (+2, concave lens and convex mirror).

  *** `lens-types`' concave_mirror ROW IS NOT CITED, AND THAT IS THE RESULT, NOT A GAP IN IT. *** The
  only sentence on the page linking a concave mirror to converging is "The focal length \(f\) of a
  concave mirror is positive, since it is a converging mirror." — and `\(f\)` is a **MathJax
  delimiter in the raw HTML**, rendered client-side as an italic *f*. The page's SOURCE text and its
  DISPLAYED text differ, so there is no reading of "verbatim" that is not a choice: quote the raw form
  and the citation embeds markup no reader ever sees; quote the rendered form and I have EDITED the
  source, which is precisely the edit that damaged the two header quotes this effort keeps finding.
  A definitional problem rather than a fetch problem, and not one to settle by preference.

  `macronutrients` is partial in the unusual direction — 3 of 4 rows were already covered and only
  ALCOHOL needed evidence. The figure appears **exactly once** on the page, parenthetically, inside a
  sentence whose subject is the pronoun "it", so the quote is widened to name alcohol. The page is
  titled "Weight loss and alcohol" and says alcohol is "high in calories" and has "empty calories" —
  phrases that *sound* supporting while never stating 7 per gram. Matching on "alcohol near calories"
  would have encoded prose that does not carry the row's value.

  Two quotes end in a figure reference ("such as those illustrated above", "as shown in part (b)")
  which the page's own sentence includes. WHERE THE PAGE'S SENTENCE BOUNDARY IS AWKWARD, THE PAGE WINS.

  SIX BLOCKER CLASSES ARE NOW KNOWN across this issue's members, every one found by attempting the
  work rather than planning it: JS-rendered pages, PDF-only evidence, evidence on a weaker-tier host
  than the table's `trust` (and `cites` has no trust field), SSL certificate failures on three NIH
  subdomains while ncbi.nlm.nih.gov works, HTTP 403, and now MathJax source/rendered ambiguity.

  All three `.query.adj` companions parse and run. 533 test binaries / 1592 tests green, clippy
  -D warnings clean. Every sentence re-extracted from its own page and verified verbatim against raw
  HTML.

