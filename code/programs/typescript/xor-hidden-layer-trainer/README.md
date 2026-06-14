# xor-hidden-layer-trainer

Runnable XOR example for the first hidden-layer lesson.

XOR is the tiny dataset where a single straight line cannot separate the outputs:

| x1 | x2 | y |
|---:|---:|---:|
| 0 | 0 | 0 |
| 0 | 1 | 1 |
| 1 | 0 | 1 |
| 1 | 1 | 0 |

This program first trains a no-hidden-layer sigmoid model long enough to show
that XOR still does not become separable. It then trains a `2 -> 2 -> 1`
network and prints checkpoints so the hidden layer can be inspected directly.

The final trace treats each neuron like a tiny service: incoming values and
weights enter, weighted terms and bias produce a raw sum, the activation emits a
new value, and backprop supplies a delta for learning.
