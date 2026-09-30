### Fixed — Text directional padding reaches Flutter Padding (#16241)

Flutter text parts now resolve `padding-top`, `padding-right`,
`padding-bottom`, and `padding-left` per edge, with longhands taking precedence
over the shorthand. The emitted `Padding` wrapper stays inside accessibility
wrappers so semantic labels and heading roles continue to describe the whole
visual node.

Fresh TaskApp generation removes the final 28 directional-padding drops plus
five text-path shorthand drops from its Flutter inventory, reducing the backend
total from 273 to 240.
