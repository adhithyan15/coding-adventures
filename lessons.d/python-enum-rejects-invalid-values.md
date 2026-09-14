---
category: Python
---

# Python Enum rejects invalid values

(`MyEnum(99)` raises `ValueError`). For "not found"/"invalid" tests use `None` or sentinels, not arbitrary ints. Use `IntEnum` if you need int compatibility.
