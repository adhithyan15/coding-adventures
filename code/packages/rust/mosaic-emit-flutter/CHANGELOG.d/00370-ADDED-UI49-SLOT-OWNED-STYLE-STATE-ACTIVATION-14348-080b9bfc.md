### Added - UI49 slot-owned style-state activation (#14348)

Flutter now activates mosstyle states owned by `one-of` slots from the
generated widget's corresponding property. Multiple enum axes follow `.mil`
slot declaration order, then existing `state-when-*` predicates take
precedence. Conditional background, foreground, border, and padding values
reach both generic styled containers and native `HostButton` `ButtonStyle`
output, including state-only styles with no base paint.

