### Fixed — container and button font weights reach Flutter (#17073)

Flutter now carries supported authored `font-weight` values through
`DefaultTextStyle` for layout containers and through the label `TextStyle` for
`HostButton`. Explicit `normal` weights override inherited emphasis, while
unsupported values remain visible in degradation reports.
