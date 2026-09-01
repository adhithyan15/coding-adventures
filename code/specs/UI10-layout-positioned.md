# UI10: Positioned Formatting and Clipping

## Status

Implemented reusable browser profile.

## Contract

Producers encode computed values in `LayoutNode.ext["positioned"]`:

- `position`: `static`, `relative`, `absolute`, `fixed`, or `sticky`
- `top`, `right`, `bottom`, `left`: finite logical lengths; omission is `auto`
- `zIndex`: an integer; omission is `auto`
- `overflowX`, `overflowY`: `visible`, `hidden`, `auto`, or `scroll`

`layout-positioned` owns tolerant decoding, diagnostics, inset resolution,
stable z-order, overflow policy, and scroll extents. Block, flex, and grid
dispatch remain consumers of that producer-neutral contract.

Relative and sticky boxes retain their normal-flow space. Absolute and fixed
boxes do not contribute to normal-flow sizing. Two specified opposing insets
stretch an auto-sized axis. Stable z-order uses source order for equal values.

Non-visible overflow emits a backend-neutral rectangular `PaintClip`, and hit
regions are intersected with the same ancestor clip. Fixed paint groups and
links remain in viewport coordinates. Sticky groups clamp to their top inset
as document scroll advances. Native and generated hosts consume those shared
paint and hit-test results without toolkit-specific CSS behavior.

## Bounded follow-ups

- transformed containing blocks and writing modes
- independent one-axis clips once paint instructions expose unbounded axes
- nested scrolling boxes and interactive scrollbars
- full CSS stacking-context isolation, floats, and fragmentation
