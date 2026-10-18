---
category: Testing & coverage
---

# A shared renderer family override needs separate no-config label and title controls

The first Timeline family semantic test plan covered a complete no-config
TemporalTitle Scene, but that exercised only the caller title font. Independent
review withheld static test-plan acceptance because an implementation could
guard the title clone correctly while still replacing the global label font.
The first compiled red run remains valid regression evidence, not full approval.

Before any consumer implementation, retain that title control and add a complete
no-config Gantt axis/title Scene with distinct caller label/title families and
ordered query vectors. The strengthened red run must pass both controls while
retaining exactly the intended family-transport failures. Shared renderer state
needs a control for every independently mutable font role; one passing title
does not prove that ordinary axis labels retain their caller font.
