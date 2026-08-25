### Added - the literal-markup gate (HL-C217, HL-C221)

- Add `literal-markup.ts`: authoring markup that survives escaping into reader-facing text — HTML entities, numeric entities, and a closed list of bare HTML tags.
- Checks **both layers**. The lesson source is where an author can act; the generated `.tex` is what the reader actually gets, and catches markup arriving from a template rather than a lesson.
- The rendered pattern is separate from the source one on purpose: the escaper turns `&` into `\&`, so grepping the book for `&nbsp;` finds nothing. That is exactly how the defect stayed invisible through three merged releases.
- Exempts the corpus's own `<!-- hl-knowledge -->` / `<!-- hl-activity -->` directives, fenced code blocks and inline code spans — a gate that flags every lesson, or every backlog entry describing the defect, gets switched off within a day. Exempt spans are blanked rather than deleted so reported line numbers stay true.
- **Blocking from the first commit**, not report-only: the corpus is clean at both layers today (0 findings across 2,885 lessons and every generated book), so there is no inherited debt to route around.
- Self-tested against a real planted defect, not just fixtures: `&nbsp;` added to a committed Spanish lesson, gate failed naming `ES-C01-hola:68 &nbsp;`, file restored, gate green.
- The corpus assertion names its findings rather than counting them, and a companion test pins that the same measurement over the same corpus plus one planted line still fires — so a clean run cannot be vacuous.

