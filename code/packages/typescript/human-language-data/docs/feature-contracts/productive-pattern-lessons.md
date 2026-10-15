# Productive pattern lessons (HL05)

`type: pattern` turns a reusable frame into canonical book-and-app data. A pattern
introduces exactly one `*-PATTERN-*` atom, declares ordered slots whose fillers are
already present in `requires.knowledge`, and gives the reader at least three distinct
guided-production instantiations. The tiny frontmatter parser represents the ordered
map as typed `patternSlots` on `ParsedLesson`:

```yaml
type: pattern
requires:
  knowledge: [ES-LEX-COMER, ES-LEX-BEBER, ES-LEX-CAFE]
introduces:
  knowledge: [ES-PATTERN-ER-FUTURE-SINGULAR]
slots:
  infinitive: [ES-LEX-COMER, ES-LEX-BEBER]
  object: [ES-LEX-CAFE]
```

The three chapter gates reject an extra introduced atom, a missing or non-list slot,
an out-of-closure filler, or fewer than three distinct guided productions. The first canonical
realization is Spanish `ES-C17-comer-futuro`: known *comer*, *beber*, and *café* fill
the new singular future frame without smuggling in vocabulary.
