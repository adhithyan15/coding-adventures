### Interaction-only XAML profiling

- Add opt-in `MOSAIC_PROFILE_INTERACTIONS_ONLY=1` to keep only handler and
  next-render measurements in the existing 4096-sample budget. Detailed setter
  and conversion phases no longer exhaust a filtered workload's reservations.
- Filter before scope creation or diagnostic access. Default profiling, JSON
  schema, pending-scope bounds and normal-exit export behavior are unchanged.
- Exercise excluded-phase starvation, exact matching, both admitted phases,
  and the pending-scope cap in the emitted C# acceptance driver.
