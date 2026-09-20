---
category: Testing & coverage
---

# Check simulator step return types before writing execution probes

A CLR control-flow probe assumed step() returned Result and called unwrap(), causing E0599. CLRSimulator::step returns CLRTrace directly and unknown opcodes panic. Inspect the API before writing probes; call step directly and preserve expected failure logs separately from successful regression tests. The lesson command also requires the exact category name Testing & coverage, not testing.
