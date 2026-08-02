# Changelog

## 1.0.0 - 2026-08-02

- Add a two-position causal next-token shift and saved decoder states.
- Add unembedding, stable-softmax, mean cross-entropy, and full gradient traces.
- Add an independent central finite-difference audit of every trainable gradient.
- Add one shared-head SGD update and a deterministic post-update loss check.
