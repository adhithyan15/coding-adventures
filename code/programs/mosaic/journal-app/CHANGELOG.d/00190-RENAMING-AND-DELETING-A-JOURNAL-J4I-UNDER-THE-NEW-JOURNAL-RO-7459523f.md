- **Renaming and deleting a journal (J4i).** Under the *New journal* row, while
  a journal is selected, a `journal-actions` row offers *Rename* (Add's
  style; it uses the field's name) and *Delete journal* (in the draft
  error's red; it moves the journal's entries to Personal). Personal is
  never offered for deletion. New slots: `can-rename-journal`,
  `can-delete-journal`. New events: `onRenameJournal` and `onDeleteJournal`
  (the contract test counts 21).
