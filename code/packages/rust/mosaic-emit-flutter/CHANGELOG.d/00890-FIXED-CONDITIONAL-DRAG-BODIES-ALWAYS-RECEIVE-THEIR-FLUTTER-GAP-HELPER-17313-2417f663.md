### Fixed — conditional drag bodies always receive their Flutter gap helper (#17313)

Flutter now decides whether to emit `_mosaicWithGap` from the same decorated
or lightweight child-list shape used by `HostDraggable` and `HostDropTarget`.
A plain body beginning with `If`/`Else` can no longer call an undefined helper,
and a decorated conditional followed by another child keeps that child inside
the evaluated vertical gap list. Native drag, keyboard, semantics, and drop
controls are unchanged.
