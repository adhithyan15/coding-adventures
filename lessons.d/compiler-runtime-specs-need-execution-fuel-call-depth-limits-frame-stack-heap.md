---
category: Compiler / VM / language pipeline
---

# Compiler runtime specs need execution fuel, call-depth limits, frame-stack/heap byte caps, and explicit captured-environment lifetime rules

before implementing recursion, closures, thunks. Source-size and AST-depth limits alone are insufficient. Either reject escaping descriptors or heap-lift captured envs.
