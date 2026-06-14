# Hidden Layer Example Suite

This suite expands the XOR hidden-layer lesson into six small problems that a
single straight line cannot handle comfortably.

Sine approximation is intentionally left out of this pass so it can get its own
focused treatment later.

## Examples

| Example | Shape | What the Hidden Layer Learns |
| --- | --- | --- |
| XNOR gate | 2 inputs -> 1 output | Two separated "same input" regions. |
| Absolute value | 1 input -> 1 output | A bend at zero, forming a V shape. |
| Piecewise pricing | 1 input -> 1 output | Soft thresholds that combine into steps. |
| Circle classifier | 2 inputs -> 1 output | Several boundaries that approximate an inside/outside region. |
| Two moons | 2 inputs -> 1 output | A curved decision boundary that cannot be drawn as one line. |
| Interaction features | 3 inputs -> 1 output | Feature combinations such as garage plus enough rooms. |

## Shared Contract

Every example uses the same two-layer primitive:

```text
inputs -> input-to-hidden weights + hidden bias -> sigmoid hidden neurons
hidden outputs -> hidden-to-output weights + output bias -> sigmoid prediction
prediction + target -> mean squared error -> backpropagation gradients
```

The language packages all exercise these examples through one training step.
That keeps the examples portable while stressing the matrix shapes that matter:

- 1 input, many hidden neurons, 1 output
- 2 inputs, many hidden neurons, 1 output
- 3 inputs, many hidden neurons, 1 output

The TypeScript visualizer goes further by letting you step and run the browser
training loop, inspect loss history, and trace one selected row through each
hidden neuron.
