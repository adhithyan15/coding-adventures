### Added - native accessible drag and drop

`HostDraggable` and `HostDropTarget` now lower to Flutter's native
`Draggable`/`DragTarget` pair. A component-instance scope registers eligible
targets for keyboard movement, pointer and keyboard releases share one accepted
drop path, disabled and accepted-kind rules are enforced before target
selection, and the generated runtime emits drag lifecycle payloads and
screen-reader announcements. Space/Enter grabs and drops, arrow keys move among
valid targets, Escape cancels, and semantic activation exposes the same flow to
assistive technologies. The native wrappers retain their authored part
decoration and spacing instead of discarding the card and target styling.

