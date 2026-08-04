# Forward Graph Lowering Fixtures v1

This directory contains the language-neutral NN29 corpus for one deterministic
forward graph lowering into normalized NN00 NeuralIR (`CANN`) and NN01 MatrixIR
plan (`CANM`).

Validate it with:

```text
python code/scripts/validate_forward_graph_lowering_labs.py
```

The fixture pins stable scheduling, value allocation, fusion provenance, a
hand-calculable row, a two-row batch, and direct/NeuralIR/MatrixIR parity.
