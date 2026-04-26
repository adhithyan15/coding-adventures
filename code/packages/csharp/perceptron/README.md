# CodingAdventures.Perceptron.CSharp

Binary perceptron classifier for small in-memory datasets.

The model trains one sigmoid neuron with binary cross-entropy. `Fit` accepts either a flat label vector or one-column label rows, then `Predict` returns a probability for each input sample.

```csharp
using CodingAdventures.Perceptron;

var model = new Perceptron(learningRate: 0.8, epochs: 5000);
model.Fit(
    new[]
    {
        new[] { 0.0, 0.0 },
        new[] { 0.0, 1.0 },
        new[] { 1.0, 0.0 },
        new[] { 1.0, 1.0 },
    },
    new[] { 0.0, 0.0, 0.0, 1.0 });

double[] probabilities = model.Predict(new[] { new[] { 1.0, 1.0 } });
```
