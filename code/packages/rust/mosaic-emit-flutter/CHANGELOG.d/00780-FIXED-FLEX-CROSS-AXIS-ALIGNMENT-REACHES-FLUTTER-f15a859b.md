### Fixed — flex cross-axis alignment reaches Flutter (#15258)

`Row` parts using `align: center-vertical` and `Column` parts using
`align: center-horizontal` now lower to `CrossAxisAlignment.center`.
The symmetric `align: center` form is supported on both axes, while
unrecognized values remain visible in dropped-style reports.

Flutter already defaults flex cross-axis alignment to center, so this makes
the authored contract explicit rather than claiming a new visual default.
Fresh TaskApp generation removes all 22 Flutter `align` drops, reducing its
Flutter style-degradation inventory from 240 to 218. Main-axis and Text
semantics remain separately tracked in #16293.
