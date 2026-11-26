---
category: Python
---

# A Python graph matrix add_edge does not coerce large integers

Do not infer that an adjacency matrix converts edge weights from its annotated
float type. Its Python list stores the value supplied to `add_edge`, just like
the adjacency-list implementation. The canonical edge `weight` property update
is a separate path that converts to `float` and can raise `OverflowError` for a
huge integer. Verify both public paths in the implementation before documenting
representation differences, and parameterize algorithm tests over both graph
representations.
