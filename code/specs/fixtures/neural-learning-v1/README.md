# Neural Learning Fixtures v1

This directory is the language-neutral numerical corpus defined by
`code/specs/NN03-neural-learning-labs.md`.

Each lab contains explicit dense-layer parameters, a tiny dataset, and expected
forward results. Training labs additionally pin the first full-batch SGD step:
the loss, gradients, updated parameters, and new loss.

```text
neural-learning-v1/
  schema.json
  CHANGELOG.md
  labs/
    00-weighted-neuron.json
    01-celsius-linear-regression.json
    02-or-sigmoid-neuron.json
    03-xor-hidden-representation.json
```

Validate the corpus:

```text
python code/scripts/validate_neural_learning_labs.py
python code/scripts/tests/test_neural_learning_labs.py
```

The files are intentionally small enough to read beside an implementation.
They are not performance benchmarks or large-model interchange files.
