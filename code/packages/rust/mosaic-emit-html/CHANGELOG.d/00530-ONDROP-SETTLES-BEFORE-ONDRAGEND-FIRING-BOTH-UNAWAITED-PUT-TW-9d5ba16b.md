- **`onDrop` settles before `onDragEnd`.** Firing both unawaited put two host
  round-trips in flight, each merging props and re-rendering, so an `onDragEnd`
  response computed from pre-drop state could land last and visually revert the
  move.
