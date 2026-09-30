### Fixed — HostButton directional padding reaches Flutter ButtonStyle (#16233)

Flutter host buttons now resolve `padding-top`, `padding-right`,
`padding-bottom`, and `padding-left` per edge, with longhands taking precedence
over the shorthand. Non-uniform values emit `EdgeInsets.fromLTRB`, while
uniform values retain the compact `EdgeInsets.all` form. State-layer shorthand
padding continues to override all four edges for the active state.

Fresh TaskApp generation removes 56 directional-padding drops from its
Flutter inventory, reducing the backend total from 329 to 273 and leaving the
28 text-widget directional-padding drops separately measurable.
