### Fixed — TaskApp fill widths reach Flutter (#17251)

Flutter now preserves `width: 100%` on text, input, and button leaves when the
parent supplies a finite horizontal constraint. Bounded non-row parents emit a
`SizedBox(width: double.infinity)`, while direct row children and matching
conditional branches reuse exactly one parent-owned `Expanded`. Other
percentages and unbounded cases remain visible in degradation reports.
