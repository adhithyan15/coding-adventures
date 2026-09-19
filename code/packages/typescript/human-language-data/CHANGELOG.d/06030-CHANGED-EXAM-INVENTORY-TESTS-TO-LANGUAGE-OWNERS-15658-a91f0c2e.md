### Changed — exam-inventory tests now have language owners

- Split the 4,173-line global exam-inventory test into twelve language-owned
  suites while preserving every coverage pin, unmapped-point audit, source note,
  and generic hostile-loader assertion.
- Kept generic semantics in one stable test and added a regression guard against
  returning language-specific committed-inventory suites to it.
- Replaced the moved suites' repeated 23-track loads with language-scoped lesson
  loads. The focused global-plus-Persian inventory run fell from about 70 seconds
  to about 7 seconds on the same host while retaining all 132 assertions.
