namespace CodingAdventures.FeatureNormalization;

public sealed record StandardScaler(IReadOnlyList<double> Means, IReadOnlyList<double> StandardDeviations);

public sealed record MinMaxScaler(IReadOnlyList<double> Minimums, IReadOnlyList<double> Maximums);

public static class FeatureNormalization
{
    public static StandardScaler FitStandardScaler(IReadOnlyList<IReadOnlyList<double>> rows)
    {
        var width = ValidateMatrix(rows);
        var means = new double[width];
        foreach (var row in rows)
        {
            for (var col = 0; col < width; col++)
            {
                means[col] += row[col];
            }
        }
        for (var col = 0; col < width; col++)
        {
            means[col] /= rows.Count;
        }

        var standardDeviations = new double[width];
        foreach (var row in rows)
        {
            for (var col = 0; col < width; col++)
            {
                var diff = row[col] - means[col];
                standardDeviations[col] += diff * diff;
            }
        }
        for (var col = 0; col < width; col++)
        {
            standardDeviations[col] = Math.Sqrt(standardDeviations[col] / rows.Count);
        }

        return new StandardScaler(means, standardDeviations);
    }

    public static IReadOnlyList<IReadOnlyList<double>> TransformStandard(IReadOnlyList<IReadOnlyList<double>> rows, StandardScaler scaler)
    {
        var width = ValidateMatrix(rows);
        if (width != scaler.Means.Count || width != scaler.StandardDeviations.Count)
        {
            throw new ArgumentException("Matrix width must match scaler width.", nameof(scaler));
        }

        var transformed = new List<IReadOnlyList<double>>(rows.Count);
        foreach (var row in rows)
        {
            var output = new double[width];
            for (var col = 0; col < width; col++)
            {
                output[col] = scaler.StandardDeviations[col] == 0.0
                    ? 0.0
                    : (row[col] - scaler.Means[col]) / scaler.StandardDeviations[col];
            }
            transformed.Add(output);
        }
        return transformed;
    }

    public static MinMaxScaler FitMinMaxScaler(IReadOnlyList<IReadOnlyList<double>> rows)
    {
        var width = ValidateMatrix(rows);
        var minimums = rows[0].ToArray();
        var maximums = rows[0].ToArray();
        foreach (var row in rows.Skip(1))
        {
            for (var col = 0; col < width; col++)
            {
                minimums[col] = Math.Min(minimums[col], row[col]);
                maximums[col] = Math.Max(maximums[col], row[col]);
            }
        }

        return new MinMaxScaler(minimums, maximums);
    }

    public static IReadOnlyList<IReadOnlyList<double>> TransformMinMax(IReadOnlyList<IReadOnlyList<double>> rows, MinMaxScaler scaler)
    {
        var width = ValidateMatrix(rows);
        if (width != scaler.Minimums.Count || width != scaler.Maximums.Count)
        {
            throw new ArgumentException("Matrix width must match scaler width.", nameof(scaler));
        }

        var transformed = new List<IReadOnlyList<double>>(rows.Count);
        foreach (var row in rows)
        {
            var output = new double[width];
            for (var col = 0; col < width; col++)
            {
                var span = scaler.Maximums[col] - scaler.Minimums[col];
                output[col] = span == 0.0 ? 0.0 : (row[col] - scaler.Minimums[col]) / span;
            }
            transformed.Add(output);
        }
        return transformed;
    }

    private static int ValidateMatrix(IReadOnlyList<IReadOnlyList<double>> rows)
    {
        if (rows.Count == 0 || rows[0].Count == 0)
        {
            throw new ArgumentException("Matrix must have at least one row and one column.", nameof(rows));
        }

        var width = rows[0].Count;
        if (rows.Any(row => row.Count != width))
        {
            throw new ArgumentException("All rows must have the same number of columns.", nameof(rows));
        }
        return width;
    }
}
