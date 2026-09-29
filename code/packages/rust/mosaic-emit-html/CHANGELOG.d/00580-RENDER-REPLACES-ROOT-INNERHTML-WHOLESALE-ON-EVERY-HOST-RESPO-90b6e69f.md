- **`render()` replaces `root.innerHTML` wholesale** on every host response, so
  drag state is module-scoped rather than stashed on elements, and the hovered
  target is tracked by *key* rather than by element reference — the element that
  key names may be a different object after a re-render.
