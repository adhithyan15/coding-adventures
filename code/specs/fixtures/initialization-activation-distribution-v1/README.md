# Initialization and Activation Distribution V1 Fixtures

This NN23 corpus compares tiny, Xavier, He, and deliberately large weight
scales under `tanh` and ReLU. The three-layer experiment fixes its inputs and
weight signs so every consumer can reproduce activation values and population
statistics without agreeing on a random-number generator.

Validate it with:

```text
python code/scripts/validate_initialization_activation_distribution_labs.py
```
