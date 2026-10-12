### Fixed — drag-and-drop bodies preserve child spacing (#17301)

`HostDraggable` and `HostDropTarget` now apply authored `gap` values to their
evaluated vertical child lists. A single authored `For` is expanded before the
spacing helper runs, so dynamic cards and events keep the intended separation;
literal single-child gaps are consumed as safe no-ops. The native pointer,
keyboard, semantics, and drop-target runtime remain unchanged.
