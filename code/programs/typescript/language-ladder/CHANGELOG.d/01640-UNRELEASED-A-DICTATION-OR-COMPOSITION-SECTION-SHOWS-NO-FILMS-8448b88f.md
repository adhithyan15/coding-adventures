## Unreleased — a dictation or composition section shows no filmstrip

- `filmstripSectionIndex` skips a Writing or Script section whose writing
  stage shows the learner no model (dictation, either composition, timed
  production), as the book now does (HL06). A Marathi heard-cue lesson
  (MR-W03-ba) shows its strip in its Script section, beside the letter the
  dictation covers; a lesson with nowhere else shows none. The book no longer
  generates a strip for those 33 lessons, so `generatedFilmstripUrl` already
  returned nothing for them; this keeps the placement rule the same as the
  book's for the next lesson that has one.
