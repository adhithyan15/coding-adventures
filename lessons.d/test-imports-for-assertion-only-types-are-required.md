---
category: Python
---

# Test imports for assertion-only types are required

— pytest doesn't pick up `LogicVar` from sibling tests; every isinstance/equality target needs its own `import`.
