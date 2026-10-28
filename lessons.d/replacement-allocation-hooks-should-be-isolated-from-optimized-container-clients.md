---
category: Testing & coverage
---

# Replacement allocation hooks should be isolated from optimized container clients

NN51's fault-injection test initially defined replacement new/delete beside
optimized vector clients. GCC inlined the matching delete-to-free implementation
and rejected it under mismatched-new-delete. Keep the standard malloc/free
replacement pair in a separate test translation unit with a small fault-control
function. Preserve strict warning flags and test the real allocation failures;
do not replace the sweep with model-side pretend failures.

The first assertion guessed that there would be more than 100 requests; GCC
actually made 93. The invariant is that every request before the first successful
operation has been failed in turn and preserved state, not an invented minimum
allocation count. Report the measured count and assert the sweep is nonempty.

Clang also rejected the literal `moved = moved` self-assignment test under
warnings-as-errors. Obtain the aliased source through an identity function to
exercise the public self-assignment branch without spelling a known accidental
self-assignment pattern; retain Clang's diagnostic unchanged.
