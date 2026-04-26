# feature_normalization

Feature scaling utilities for machine-learning examples.

This package provides standard scaling and min-max scaling for small numeric
feature matrices. Constant columns map to `0.0`, which keeps training examples
stable when a feature does not vary in a short batch.
