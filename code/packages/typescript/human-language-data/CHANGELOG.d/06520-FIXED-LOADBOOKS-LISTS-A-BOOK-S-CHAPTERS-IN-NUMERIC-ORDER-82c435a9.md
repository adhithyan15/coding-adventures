### Fixed — loadBooks lists a book's chapters in numeric order

`loadBooks` read each book's `chapters/` directory in string order. Once a
track passed ninety-nine chapters, `ch100-...` sorted between `ch10-...` and
`ch11-...`. Consumers read that list as the book's chapter order, so it was
wrong for French, German, Italian and Portuguese. It now sorts by chapter
number, the same rule `chapterInputsFor` already applies when it writes
`book.tex`, so printed books were never affected. The Russian integration pin,
chapters 1 to 135, now covers the order.
