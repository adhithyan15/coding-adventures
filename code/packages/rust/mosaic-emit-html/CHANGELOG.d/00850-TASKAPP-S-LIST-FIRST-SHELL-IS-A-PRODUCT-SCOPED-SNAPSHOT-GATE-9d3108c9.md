- **TaskApp's List-first shell is a product-scoped snapshot gate (#16120).**
  The emitter test lane compiles TaskApp's real interface, package-expanded
  layout, and light theme. The acceptance pins the task composer, repeated task
  row, and SegmentedControl view-switch structure, plus the explicit
  static-HTML dropped-event markers. It intentionally makes no runtime,
  interaction, or release-artifact claim.

