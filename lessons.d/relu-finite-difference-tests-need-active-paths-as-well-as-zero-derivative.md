---
category: Testing & coverage
---

# ReLU finite-difference tests need active paths as well as zero-derivative boundaries

NN45's first unequal-width ReLU finite-difference network happened to have dead
output paths: every analytical gradient was zero, and numerical differences
also passed at zero. That checked the inactive derivative but did not exercise
useful multilayer backpropagation.

For ReLU's full-parameter central differences, choose positive weights and
biases away from the kink, and assert every gradient is nonzero before comparing
it. Keep the separate zero-input test for the specified derivative at zero.
Inspect gradient activity, not just the number of coordinates visited.
