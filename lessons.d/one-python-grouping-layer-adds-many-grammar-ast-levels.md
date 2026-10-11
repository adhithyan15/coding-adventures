---
category: Compiler / VM / language pipeline
---

# One Python grouping layer adds many grammar AST levels

An oracle fixture `print((1.0 + (2.0 * 3.0)))` exceeded LANG79's existing
64-level direct-AST budget. Python precedence wrappers add many tree levels
per explicit grouping layer, so small source nesting can still cross the
pilot's structural bound. Host Python accepted the source; the native pilot
correctly rejected it before lowering under its narrower contract.

Use fixtures within the declared AST bounds for semantic comparisons and
report depth rejection separately. Preserve the cap unless a separate,
evidence-backed bound change is intended; removing a resource guard to make
a conformance fixture pass would change the task's scope.
