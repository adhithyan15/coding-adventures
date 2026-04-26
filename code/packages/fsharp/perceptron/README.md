# CodingAdventures.Perceptron.FSharp

Binary perceptron classifier for small in-memory datasets.

The model trains one sigmoid neuron with binary cross-entropy. `Fit` accepts either a flat label vector or one-column label rows, then `Predict` returns a probability for each input sample.

```fsharp
open CodingAdventures.Perceptron.FSharp

let model = Perceptron(0.8, 5000)

model.Fit(
    [| [| 0.0; 0.0 |]; [| 0.0; 1.0 |]; [| 1.0; 0.0 |]; [| 1.0; 1.0 |] |],
    [| 0.0; 0.0; 0.0; 1.0 |])

let probabilities = model.Predict [| [| 1.0; 1.0 |] |]
```
