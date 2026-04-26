using CodingAdventures.GradientDescent;

namespace CodingAdventures.GradientDescent.Tests;

public sealed class GradientDescentTests
{
    [Fact]
    public void SgdAppliesWeightUpdates()
    {
        var updated = GradientDescent.Sgd([1.0, -0.5, 2.0], [0.1, -0.2, 0.0], 0.1);

        Assert.Equal(0.99, updated[0], 9);
        Assert.Equal(-0.48, updated[1], 9);
        Assert.Equal(2.0, updated[2], 9);
    }

    [Fact]
    public void SgdRejectsInvalidInputs()
    {
        Assert.Throws<ArgumentNullException>(() => GradientDescent.Sgd(null!, [1.0], 0.1));
        Assert.Throws<ArgumentNullException>(() => GradientDescent.Sgd([1.0], null!, 0.1));
        Assert.Throws<ArgumentException>(() => GradientDescent.Sgd([], [], 0.1));
        Assert.Throws<ArgumentException>(() => GradientDescent.Sgd([1.0], [], 0.1));
    }
}
