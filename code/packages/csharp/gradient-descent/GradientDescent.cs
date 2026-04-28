namespace CodingAdventures.GradientDescent;

public static class GradientDescent
{
    public static IReadOnlyList<double> Sgd(IReadOnlyList<double> weights, IReadOnlyList<double> gradients, double learningRate)
    {
        ArgumentNullException.ThrowIfNull(weights);
        ArgumentNullException.ThrowIfNull(gradients);

        if (weights.Count == 0 || weights.Count != gradients.Count)
        {
            throw new ArgumentException("Weights and gradients must have the same non-zero length.");
        }

        var updated = new double[weights.Count];
        for (var index = 0; index < weights.Count; index++)
        {
            updated[index] = weights[index] - learningRate * gradients[index];
        }

        return updated;
    }
}
