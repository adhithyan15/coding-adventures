# Native Go Neural Framework

NN41 independently implements NN37 dense training using the existing native
Go matrix package. It is not a binding or a teaching-only warm start. Layers
support different widths and linear, sigmoid, tanh and ReLU activations.

```go
model, err := neuralframework.Seeded(
    []int{2, 4, 1},
    []neuralframework.Activation{neuralframework.Tanh, neuralframework.Sigmoid}, 7,
)
// Check err before use. Inputs and targets are sample-by-feature matrices.
if err != nil { panic(err) }
losses, err := model.Fit(inputs, targets, 3000, 1)
```

`New`, `Seeded`, `Predict`, `Gradients`, `TrainStep`, and `Fit` construct,
inspect and train full networks. `Step`, `ToCheckpoint`, `FromCheckpoint`,
`ToJSON`, and `FromJSON` provide owned portable V1 state. Arrays never alias
model state; invalid updates commit neither parameters nor optimizer step.
`Fit` returns initial plus post-update losses, including one loss for zero
epochs. Networks require external synchronization for concurrent mutation.

JSON readers reject missing/unknown fields, null/boolean numeric cells,
malformed shapes and unsafe step counts. Text loading is bounded to 4 MiB;
the owned record API operates on already-allocated typed data. Checkpoints
contain data only, never executable code or paths. The deterministic uint32
LCG is for reproducibility, not cryptography.

From this module directory:

```sh
go test ./... -cover
go vet ./...
go build ./...
go run ./cmd/xor
python ../../../scripts/validate_neural_framework_go.py
```

`go run ./cmd/xor --resume` accepts only a checkpoint JSON document on stdin
and advances exactly one step. The receipt contains `checkpoint`,
`predictions`, `initial_loss`, and `final_loss`. This proves native training
and native portable reload/resume. NN42's separate conformance runner
`python code/scripts/validate_neural_framework_interchange.py` (repository root,
with Node/TypeScript dependencies, Cargo and Go installed) executes all twelve
directed exchanges among Go, Python, TypeScript and Rust through step 3002.
Selected required CI must pass before recording conformance delivery.

Tests consume the shared paper/seed/XOR fixture and all 49 NN40 checkpoint
cases, central finite differences for every parameter/activation, ownership,
overflow and atomicity boundaries. Library coverage target is at least 95%.
See [NN41](../../../specs/NN41-native-go-dense-framework.md) and the
[lesson](../../../learning/machine-learning/native-go-dense-framework.md).
