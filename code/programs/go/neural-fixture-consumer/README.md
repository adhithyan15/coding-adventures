# Go neural fixture consumer

This dependency-free Go program reads the NN03 weighted-neuron JSON fixture,
recomputes its native binary64 forward pass, and writes one strict JSON receipt.

```bash
go run . --fixture ../../../specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json
go test ./...
```

It is the compiled, garbage-collected lane in NN34. It does not call Python or
copy a stored receipt.
