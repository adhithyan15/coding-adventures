using Activation = CodingAdventures.ActivationFunctions.ActivationFunctions;

namespace CodingAdventures.ActivationFunctions.Tests;

public sealed class ActivationFunctionsTests
{
    private static void AssertClose(double expected, double actual, double tolerance = 1e-12)
    {
        Assert.True(
            Math.Abs(expected - actual) <= tolerance,
            $"Expected {expected:R} but got {actual:R}.");
    }

    [Fact]
    public void LinearAndItsDerivativeMatchTheIdentityDefinition()
    {
        AssertClose(-3.0, Activation.Linear(-3.0));
        AssertClose(0.0, Activation.Linear(0.0));
        AssertClose(5.0, Activation.Linear(5.0));

        AssertClose(1.0, Activation.LinearDerivative(-3.0));
        AssertClose(1.0, Activation.LinearDerivative(0.0));
        AssertClose(1.0, Activation.LinearDerivative(5.0));
    }

    [Fact]
    public void SigmoidMatchesTheSpecVectors()
    {
        AssertClose(0.5, Activation.Sigmoid(0.0));
        AssertClose(0.7310585786300049, Activation.Sigmoid(1.0));
        AssertClose(0.2689414213699951, Activation.Sigmoid(-1.0));
        AssertClose(0.9999546021312976, Activation.Sigmoid(10.0));
        AssertClose(0.0, Activation.Sigmoid(-710.0));
        AssertClose(1.0, Activation.Sigmoid(710.0));
    }

    [Fact]
    public void SigmoidDerivativeMatchesTheSpecVectors()
    {
        AssertClose(0.25, Activation.SigmoidDerivative(0.0));
        AssertClose(0.19661193324148185, Activation.SigmoidDerivative(1.0));
        AssertClose(0.00004539580773595167, Activation.SigmoidDerivative(10.0));
    }

    [Fact]
    public void ReluAndItsDerivativeMatchThePiecewiseDefinition()
    {
        AssertClose(5.0, Activation.Relu(5.0));
        AssertClose(0.0, Activation.Relu(-3.0));
        AssertClose(0.0, Activation.Relu(0.0));

        AssertClose(1.0, Activation.ReluDerivative(5.0));
        AssertClose(0.0, Activation.ReluDerivative(-3.0));
        AssertClose(0.0, Activation.ReluDerivative(0.0));
    }

    [Fact]
    public void LeakyReluAndItsDerivativeKeepTheNegativeSlope()
    {
        AssertClose(5.0, Activation.LeakyRelu(5.0));
        AssertClose(-0.03, Activation.LeakyRelu(-3.0));
        AssertClose(0.0, Activation.LeakyRelu(0.0));

        AssertClose(1.0, Activation.LeakyReluDerivative(5.0));
        AssertClose(0.01, Activation.LeakyReluDerivative(-3.0));
        AssertClose(0.01, Activation.LeakyReluDerivative(0.0));
    }

    [Fact]
    public void TanhAndItsDerivativeMatchTheReferenceValues()
    {
        AssertClose(0.0, Activation.Tanh(0.0));
        AssertClose(0.7615941559557649, Activation.Tanh(1.0));
        AssertClose(-0.7615941559557649, Activation.Tanh(-1.0));

        AssertClose(1.0, Activation.TanhDerivative(0.0));
        AssertClose(0.41997434161402614, Activation.TanhDerivative(1.0));
    }

    [Fact]
    public void SoftplusAndItsDerivativeMatchTheReferenceValues()
    {
        AssertClose(0.6931471805599453, Activation.Softplus(0.0));
        AssertClose(1.3132616875182228, Activation.Softplus(1.0));
        AssertClose(0.31326168751822286, Activation.Softplus(-1.0));
        Assert.True(Activation.Softplus(1000.0) > 999.0);

        AssertClose(0.5, Activation.SoftplusDerivative(0.0));
        AssertClose(Activation.Sigmoid(1.0), Activation.SoftplusDerivative(1.0));
        AssertClose(Activation.Sigmoid(-1.0), Activation.SoftplusDerivative(-1.0));
    }

    [Fact]
    public void SymmetryAndRangePropertiesHoldForRepresentativeSamples()
    {
        foreach (var sample in new[] { -6.0, -1.5, -0.5, 0.5, 1.5, 6.0 })
        {
            var sigmoid = Activation.Sigmoid(sample);
            Assert.InRange(sigmoid, 0.0, 1.0);
            AssertClose(sigmoid, 1.0 - Activation.Sigmoid(-sample), 1e-10);

            var tanh = Activation.Tanh(sample);
            Assert.InRange(tanh, -1.0, 1.0);
            AssertClose(tanh, -Activation.Tanh(-sample), 1e-10);

            Assert.True(Activation.SigmoidDerivative(sample) >= 0.0);
            Assert.True(Activation.ReluDerivative(sample) >= 0.0);
            Assert.True(Activation.LeakyReluDerivative(sample) >= 0.0);
            Assert.True(Activation.TanhDerivative(sample) >= 0.0);
            Assert.True(Activation.SoftplusDerivative(sample) >= 0.0);
        }
    }
}
