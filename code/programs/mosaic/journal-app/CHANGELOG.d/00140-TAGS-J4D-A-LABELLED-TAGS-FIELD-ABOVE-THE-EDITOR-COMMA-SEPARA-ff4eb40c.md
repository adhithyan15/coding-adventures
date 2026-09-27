- **Tags (J4d).** A labelled *Tags* field above the editor (comma-separated,
  saved by Save), each row's tags as its `meta`, and, when any entry has a
  tag, a vertical `SegmentedControl` of options like "#travel (3)" (the first
  `SegmentedControl` in Journal; vertical because the pane is 300px). An
  option filters every list, and selecting it again clears the filter. New
  slots: `draft-tags`, `tag-options`, `selected-tag-index`, `has-tags`. New
  events: `onTagsChange`, `onSelectTag`.
