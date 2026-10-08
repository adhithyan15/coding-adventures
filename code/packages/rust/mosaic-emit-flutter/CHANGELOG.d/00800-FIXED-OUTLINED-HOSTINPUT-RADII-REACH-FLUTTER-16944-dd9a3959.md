### Fixed — outlined HostInput radii reach Flutter (#16944)

Flutter `HostInput` parts with an authored solid outline now carry strict,
non-negative pixel `border-radius` values into
`OutlineInputBorder.borderRadius`. Radius-only inputs and values that cannot be
lowered without layout context remain unchanged and visible in the style-drop
report.

Fresh TaskApp generation removes 11 Flutter `border-radius` drops, reducing
that property's ratchet from 26 to 15 and the measured Flutter degradation
inventory from 209 to 198.
