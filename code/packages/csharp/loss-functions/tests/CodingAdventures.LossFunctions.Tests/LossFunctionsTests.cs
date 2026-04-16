using Loss = CodingAdventures.LossFunctions.LossFunctions;

namespace CodingAdventures.LossFunctions.Tests;

public sealed class LossFunctionsTests
{
    private static void AssertClose(double expected, double actual, double tolerance = 1e-9)
    {
        Assert.True(
            Math.Abs(expected - actual) <= tolerance,
            $"Expected {expected:R} but got {actual:R}.");
    }

    private static void AssertClose(double[] expected, double[] actual, double tolerance = 1e-9)
    {
        Assert.Equal(expected.Length, actual.Length);
        for (var i = 0; i < expected.Length; i++)
        {
            AssertClose(expected[i], actual[i], tolerance);
        }
    }

    [Fact]
    public void CoreLossFunctionsMatchTheSpecVectors()
    {
        AssertClose(0.02, Loss.Mse([1.0, 0.0, 0.0], [0.9, 0.1, 0.2]));
        AssertClose(0.13333333333333333, Loss.Mae([1.0, 0.0, 0.0], [0.9, 0.1, 0.2]));
        AssertClose(0.14462152754328741, Loss.Bce([1.0, 0.0, 1.0], [0.9, 0.1, 0.8]));
        AssertClose(0.07438118377140324, Loss.Cce([1.0, 0.0, 0.0], [0.8, 0.1, 0.1]));
    }

    [Fact]
    public void IdenticalVectorsHaveZeroRegressionLoss()
    {
        var values = new[] { 1.0, 0.0, 0.5 };
        AssertClose(0.0, Loss.Mse(values, values));
        AssertClose(0.0, Loss.Mae(values, values));
    }

    [Fact]
    public void DerivativesMatchTheReferenceCalculations()
    {
        AssertClose([-0.2, 0.2], Loss.MseDerivative([1.0, 0.0], [0.8, 0.2]));
        AssertClose([-1.0 / 3.0, 1.0 / 3.0, 0.0], Loss.MaeDerivative([1.0, 0.0, 0.5], [0.8, 0.2, 0.5]));
        AssertClose([-0.625, 0.625], Loss.BceDerivative([1.0, 0.0], [0.8, 0.2]));
        AssertClose([-0.625, 0.0], Loss.CceDerivative([1.0, 0.0], [0.8, 0.2]));
    }

    [Fact]
    public void InvalidShapesThrowForEveryLoss()
    {
        Assert.Throws<ArgumentException>(() => Loss.Mse([1.0], [0.9, 0.1]));
        Assert.Throws<ArgumentException>(() => Loss.Mae([1.0], [0.9, 0.1]));
        Assert.Throws<ArgumentException>(() => Loss.Bce([1.0], [0.9, 0.1]));
        Assert.Throws<ArgumentException>(() => Loss.Cce([1.0], [0.9, 0.1]));
    }

    [Fact]
    public void EmptyVectorsThrowForEveryLoss()
    {
        Assert.Throws<ArgumentException>(() => Loss.Mse([], []));
        Assert.Throws<ArgumentException>(() => Loss.Mae([], []));
        Assert.Throws<ArgumentException>(() => Loss.Bce([], []));
        Assert.Throws<ArgumentException>(() => Loss.Cce([], []));
    }

    [Fact]
    public void CrossEntropyClampsBoundaryProbabilitiesToStayFinite()
    {
        var bce = Loss.Bce([1.0, 0.0], [1.0, 0.0]);
        var cce = Loss.Cce([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]);

        Assert.True(double.IsFinite(bce));
        Assert.True(double.IsFinite(cce));
    }
}
