### Fixed — Qt TaskApp milestone rotations

- Preserve centered finite degree rotations on Qt painted rectangles, including
  ordered state overrides, without altering layout dimensions (#17675).
- Verify both authored TaskApp themes and tighten the Qt style-drop allowance
  from 83 to 81; unsupported transforms remain explicit debt.
