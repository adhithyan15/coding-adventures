## Slug-based lesson ids + el/la before día

- **Stopped numbering lessons.** Ordinal ids (`ES-C01-L03`) forced a
  renumber cascade on every insertion. Lessons now use **stable slug ids**
  (`ES-C01-dia`), matching filenames; prerequisites/cross-links reference
  slugs; prose refers to other lessons by word ("the *bien* lesson"), not
  "Lesson N". Order lives where a tool counts it: the **book** (LaTeX
  auto-numbers chapters/sections; `\ref` cross-refs self-correct on insert)
  and `session-map.md`. HL00 schema updated to document this.
- **Added an *el* / *la* lesson before *día*.** Gender should be understood
  before nouns, so the two articles — and the grammatical-gender concept,
  traced to Latin demonstratives *ille*/*illa* (which also gave *él*/*ella*)
  — now get their own lesson ahead of the first noun. *día* is slimmed to
  *apply* that gender rather than introduce it from scratch.
- Verified the book still compiles clean (20 pages) with the section
  inserted — LaTeX renumbered and the `\S\ref` cross-references resolved
  automatically.

