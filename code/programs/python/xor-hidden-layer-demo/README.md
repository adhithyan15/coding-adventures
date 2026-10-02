# xor-hidden-layer-demo

A runnable demonstration of why XOR needs a hidden layer.

XOR's true cases, `(0,1)` and `(1,0)`, sit on opposite corners of the unit
square, so no single straight line separates them from `(0,0)` and `(1,1)`. A
network with no hidden layer draws exactly one line, so it cannot learn XOR no
matter how long it trains. One hidden layer gives it two lines to combine.

`main.py` shows this in three steps, using the repo's own neural-network
packages:

1. A `SingleLayerNetwork` (no hidden layer) trains for 50,000 epochs and still
   misclassifies XOR.
2. A `TwoLayerNetwork` with one hidden layer, from warm-start parameters,
   classifies all four inputs. It also prints the hidden activations, so you
   can see the two "lines" it learned.
3. The same XOR network, built as a graph with `neural-network`, is compiled to
   bytecode by `neural-graph-vm` and run on its VM.

## How it fits in the stack

| Package | Role here |
| --- | --- |
| `single-layer-network` | the model that fails |
| `two-layer-network` | the model that succeeds |
| `multi-directed-graph`, `neural-network` | the network as a graph |
| `neural-graph-vm` | compiles the graph to bytecode and runs it |

## Running

```sh
PYTHONPATH=../../../packages/python/two-layer-network/src:../../../packages/python/single-layer-network/src:../../../packages/python/multi-directed-graph/src:../../../packages/python/neural-network/src:../../../packages/python/neural-graph-vm/src python3 main.py
```

`BUILD` also runs each dependency's tests first. `BUILD_windows` runs the same
steps with cmd syntax (`set "PYTHONPATH=a;b" && ...`).
