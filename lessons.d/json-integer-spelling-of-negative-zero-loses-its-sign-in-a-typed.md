---
category: Testing & coverage
---

# JSON integer spelling of negative zero loses its sign in a typed number parser

NN48's expanded bit-exact parameter round-trip tests failed in every available
numeric locale: %.17g emits negative zero as -0, and NN47 correctly classifies
that lexical integer as integer zero, whose double conversion is positive zero.
The neural serializer must emit -0.0 for a negative-zero parameter. Do not weaken
the reusable parser's integer/float distinction or metadata admission to fix a
consumer's spelling. Keep extreme normal/subnormal values and signed-zero bits
in the native round-trip regression, and rerun every compiler/sanitizer lane.
