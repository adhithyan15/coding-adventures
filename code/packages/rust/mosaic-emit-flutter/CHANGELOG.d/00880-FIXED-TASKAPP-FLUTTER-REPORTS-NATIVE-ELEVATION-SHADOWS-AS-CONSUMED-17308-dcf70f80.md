### Fixed — TaskApp Flutter reports native elevation shadows as consumed (#17308)

When a supported Mosaic `elevation` token already emits native Flutter depth,
its paired non-inset CSS `box-shadow` now leaves the style-degradation report.
Raw shadows without elevation and inset drawing effects remain explicit debt.
TaskApp's fresh Flutter inventory therefore drops from 79 to 69 without any
change to the generated widgets or platform coverage.
