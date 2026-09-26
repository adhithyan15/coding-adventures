- **Journals (J4f).** Under the tags, a second vertical `SegmentedControl`
  mount switches between "All journals" and each journal (after the tags, so
  the tag options keep their part names). Under it, a *New journal* field and
  an *Add* button, in the search bar's styles, and a red line when a name is
  refused. New entries go to the selected journal. New slots:
  `journal-options`, `selected-journal-index`, `new-journal-name`,
  `journal-error`. New events: `onSelectJournal`, `onNewJournalNameChange`,
  `onAddJournal`. The event contract test now counts 18.
