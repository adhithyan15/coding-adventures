---
category: C#
---

# Profiler acceptance processes must pin inherited mode settings

A new interaction-only mode can accidentally suppress existing property-phase acceptance records when the test subprocess inherits that mode from the caller. Set the mode explicitly for each profiler scenario, including the older full-phase props probe. Review caught this omission before publication. Keep the filtered and full-phase cases in separate processes because the host reads configuration once at initialization.
