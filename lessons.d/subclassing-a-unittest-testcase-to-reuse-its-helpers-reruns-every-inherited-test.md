---
category: Python
---

# Subclassing a unittest TestCase to reuse its helpers reruns every inherited test

LANG84 phase 2b added `SubsetsTest` and `MatrixTest` to
`code/scripts/tests/test_release_inventory.py`. To reuse the `write`,
`errors` and `declare` helpers, they first subclassed the existing
`InventoryTest` (and `MatrixTest` subclassed `SubsetsTest`). The run looked
fine: it reported `Ran 101 tests ... OK`.

It should have reported 43. unittest collects every `test_*` method a class
has, inherited ones included, so all of `InventoryTest`'s tests ran three
times, once per class. Nothing failed. The suite was slower and its count
was inflated, and a test that depends on per-class state could have passed
under one class and been hidden by the others.

The fix is a helper-only base class with no `test_*` methods
(`class Fixture(unittest.TestCase)`), with every concrete test class
inheriting from it directly.

What to do differently: put shared helpers in a base class (or a mixin that
does not inherit from `TestCase`) that has no tests of its own. When you add
test classes, compare the test count against the number of tests you wrote.
A count that is a multiple of the expected one points to inherited tests.
