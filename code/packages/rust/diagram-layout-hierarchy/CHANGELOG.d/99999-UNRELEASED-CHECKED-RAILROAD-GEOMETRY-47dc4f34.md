## Unreleased — checked Railroad geometry and stable sequence extents

Add typed try_layout_railroad spacing/derived-geometry refusals and preserve
layout_railroad as a documented infallible compatibility wrapper. Sequence
width is the measured item sum plus n-1 gaps, avoiding trailing-gap cancellation.
Controls cover84 typed refusals,96 geometry cells,40 empty/singleton extents,
ordered distinct-leaf flat/nested paths and two-rule gaps. Complete affected
14-package suites865/0/0 and strict lint pass. Miri DEFERRED/PENDING; arbitrary
recursive IR depth/remeasure resource bounds remain open.
