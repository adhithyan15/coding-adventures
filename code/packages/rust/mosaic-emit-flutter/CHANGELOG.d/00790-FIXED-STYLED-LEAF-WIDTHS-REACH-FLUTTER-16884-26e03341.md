### Fixed — styled leaf widths reach Flutter (#16884)

Fixed pixel `width` values on styled `Text`, `HostInput`/`Input`, and
`HostButton` nodes now lower through a shared `SizedBox` wrapper. A fixed-width
input directly inside a `Row` also keeps that authored bound instead of being
unconditionally expanded.

Fresh TaskApp generation removes nine Flutter `width` drops, reducing its
Flutter style-degradation inventory from 218 to 209. Percentage widths remain
reported until they have a constraint-aware flex lowering.
