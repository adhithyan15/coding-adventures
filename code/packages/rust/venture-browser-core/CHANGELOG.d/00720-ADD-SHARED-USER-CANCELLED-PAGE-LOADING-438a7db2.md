- Add a shared Stop transaction that settles pending stylesheets and images,
  invalidates late completions, preserves the committed page and history, and
  emits the ordered scheduler cancellation list through the native bridge.
