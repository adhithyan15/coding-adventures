using Norm = CodingAdventures.FeatureNormalization.FeatureNormalization;

namespace CodingAdventures.FeatureNormalization.Tests;

public sealed class FeatureNormalizationTests
{
    private static readonly IReadOnlyList<IReadOnlyList<double>> Rows =
    [
        [1000.0, 3.0, 1.0],
        [1500.0, 4.0, 0.0],
        [2000.0, 5.0, 1.0],
    ];

    private static void AssertClose(double expected, double actual) =>
        Assert.True(Math.Abs(expected - actual) <= 1e-9, $"Expected {expected:R}, got {actual:R}.");

    [Fact]
    public void StandardScalerCentersAndScalesColumns()
    {
        var scaler = Norm.FitStandardScaler(Rows);
        AssertClose(1500.0, scaler.Means[0]);
        AssertClose(4.0, scaler.Means[1]);

        var transformed = Norm.TransformStandard(Rows, scaler);
        AssertClose(-1.224744871391589, transformed[0][0]);
        AssertClose(0.0, transformed[1][0]);
        AssertClose(1.224744871391589, transformed[2][0]);
    }

    [Fact]
    public void MinMaxScalerMapsColumnsToUnitRange()
    {
        var transformed = Norm.TransformMinMax(Rows, Norm.FitMinMaxScaler(Rows));
        Assert.Equal([0.0, 0.0, 1.0], transformed[0]);
        Assert.Equal([0.5, 0.5, 0.0], transformed[1]);
        Assert.Equal([1.0, 1.0, 1.0], transformed[2]);
    }

    [Fact]
    public void ConstantColumnsMapToZero()
    {
        IReadOnlyList<IReadOnlyList<double>> rows = [[1.0, 7.0], [2.0, 7.0]];
        var standard = Norm.TransformStandard(rows, Norm.FitStandardScaler(rows));
        var minMax = Norm.TransformMinMax(rows, Norm.FitMinMaxScaler(rows));
        AssertClose(0.0, standard[0][1]);
        AssertClose(0.0, minMax[0][1]);
    }
}
