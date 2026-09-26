- The document-shell pass decides whether a node before `<frameset>` is
  ignorable with the parse's own scripting flag; it had hard-coded scripting
  on, which the new default made wrong for a `<noscript>` there.

