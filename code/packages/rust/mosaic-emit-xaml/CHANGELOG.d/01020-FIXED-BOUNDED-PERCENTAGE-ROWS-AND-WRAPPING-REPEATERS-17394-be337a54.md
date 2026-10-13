### Fixed — bounded percentage rows and wrapping repeaters (#17394)

- Complete direct-child Row width-percentage groups now lower to proportional Grid star
  definitions, and a wrapping Row with one repeated equal-fraction child lowers
  to a bounded `UniformGridLayout`. Layout-aware degradation reporting consumes
  those properties only for the exact supported shapes. Unsupported percentage
  and wrapping forms remain reported. (#17394)
