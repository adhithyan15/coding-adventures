## Unreleased — the skipped-table voice test reads a table the corpus still refuses

- **Tests:** `tests/voicescript.test.ts` checked that the narrator's "There is a
  table here I cannot read to you" notice is spoken. It did this on French
  chapter 1, whose four-column Latin/Spanish/French/English sound-change table
  was refused. That table is now three labelled columns (Latin | Spanish /
  French | English), and the narrator reads it, so the chapter has no notice
  left. The test now reads Spanish chapter 46. Its eighteen-form -ar/-er/-ir
  grid is one of the two tables the corpus still refuses on purpose. No source
  file changed.
