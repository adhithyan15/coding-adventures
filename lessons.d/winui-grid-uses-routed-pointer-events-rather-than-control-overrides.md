---
category: C#
---

# WinUI Grid uses routed pointer events rather than Control overrides

A generated table derives from Grid, which has no OnPointerWheelChanged override. Native compilation failed CS0115, followed by misleading XAML unknown-type errors. Subscribe to PointerWheelChanged and read the first compiler error before investigating the downstream XAML cascade.
