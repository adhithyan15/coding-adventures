### Added — `accessibility.button-selected-unsupported` (UI86, #15420)

`ignored_native_property` has a new `("HostButton", "selected")` arm. It asks
each native emitter's `host_button_selected_is_native`, which is the same
function that emitter lowers with, so the report cannot drift from the output.

**Current state:**
- **Compose, Flutter, Qt, SwiftUI and XAML** lower every accepted shape (XAML
  since #15463).
- **A string or number** is reported everywhere.

**Test:** `host_button_selected_is_native_where_it_is_lowered` compiles a
package with four shapes: a slot, a literal, a loop binding, and
`( i == selectedIndex )` inside `For`.
- **All five backends:** it asserts the package is native-complete, and that
  a string value is reported.

**Mutation-checked:** forcing each of the four predicates to `false` in turn
fails the test.

