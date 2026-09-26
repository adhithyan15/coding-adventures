### Fixed -- a relative length collapsed the subtree to zero (#15213)

`fixed_pixel_length` exists because `parse_pixel_value`'s "unreadable -> 0"
fallback turns a size into a zero-size box and eats whatever is inside it.
It declined `%` and (since #15160) negatives, and let **every other relative
form** through to that exact fallback -- `100vh`, `max-content`, `auto`,
`calc(...)`, `rem`, `em`.

Two writers also bypassed it entirely, calling `parse_pixel_value` for
width and height directly: `emit_styled_box` and
`style_prop_to_container_arg`.

**Three subtrees were collapsed in the shipped products**, measured on the
emitted Dart:

| product | part | authored | emitted |
| --- | --- | --- | --- |
| visicalc | `Column [workbook]` -- **the root** | `height: 100vh` | `Container(height: 0)` |
| task-app | two calendar cells | `width: 100%` | `Container(width: 0)` |

VisiCalc's is the root of the whole component, so the Flutter build rendered
**nothing at all**.

The parse is a positive test now -- a plain pixel value, optionally suffixed
`px` -- so a unit nobody has thought of yet declines by default rather than
collapsing a subtree. Declining means emitting **no size argument**, leaving
the child to size itself, which is what `width: 100%` wants in the first
place.

Two things found in security review, both inside this change:

- **Declining a width removes a bound.** `Expanded` was gated only on
  `direct_row_child`, never on `direct_row_accepts_flex` -- despite a
  comment thirty lines below claiming it was. So removing the `SizedBox`
  that had been a subtree's only width bound could leave an `Expanded`
  measured unbounded, which throws
  `RenderFlex children have non-zero flex but incoming width constraints
  are unbounded`. That would have turned a silently-blank subtree into a
  thrown layout error -- a different failure, not a fixed one. The gate now
  matches what the comment always claimed. No product output changes.
- **The charset gate is not a parse.** `1-2`, `1.2.3` and `.` pass the
  character check and still fail to parse, and delegating those to
  `parse_pixel_value` answered `0` -- the very collapse this function
  exists to prevent, through a narrower door. The result is derived from a
  real parse now.

**Flutter only.** Compose, Qt, SwiftUI, XAML, React and HTML all decline or
pass through relative lengths correctly; measured on VisiCalc, whose root
carries `height: 100vh`. The corpus authors 86 relative lengths, so most
already reached writers that handled them -- these three reached ones that
did not.

