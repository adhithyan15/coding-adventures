### Fixed — Text part typography reaches Flutter TextStyle (#15285)

Flutter `Text` nodes now apply their part's authored `color`, `font-size`,
`font-family`, and `font-weight` through `TextStyle` even when they are not
inside a table and do not carry a live numeric font-size prop. TaskApp's
measured Flutter style-degradation inventory falls from 463 to 371 entries,
including `color` from 72 to 3 and `font-weight` from 39 to 16.

The toolkit native-completeness gate now collects failures from every backend
before asserting, so an earlier Qt failure cannot hide Flutter regressions.
