### Fixed — button and container font sizes reach Flutter TextStyle (#16201)

Flutter `HostButton` labels now apply finite, non-negative stylesheet
`font-size` values to their generated `Text` children. Text-bearing `Row`,
`Column`, and `Stack` parts project the same values through
`DefaultTextStyle`, while invalid values remain visible in degradation reports
instead of becoming zero-size text.

Fresh TaskApp generation removes all 42 remaining Flutter `font-size` drops,
reducing its Flutter inventory from 371 to 329 and the five-native-backend
total from 986 to 944. The Journal and shared TaskApp gates retire their
completed font-size allowance and baseline; the toolkit keeps its narrower
allowance for other primitive families.
