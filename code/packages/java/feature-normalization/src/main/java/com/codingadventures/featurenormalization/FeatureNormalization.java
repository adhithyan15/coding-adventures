package com.codingadventures.featurenormalization;

public final class FeatureNormalization {
    private FeatureNormalization() {}

    public record StandardScaler(double[] means, double[] standardDeviations) {}

    public record MinMaxScaler(double[] minimums, double[] maximums) {}

    public static StandardScaler fitStandardScaler(double[][] rows) {
        int width = validateMatrix(rows);
        double[] means = new double[width];

        for (double[] row : rows) {
            for (int col = 0; col < width; col++) {
                means[col] += row[col];
            }
        }
        for (int col = 0; col < width; col++) {
            means[col] /= rows.length;
        }

        double[] standardDeviations = new double[width];
        for (double[] row : rows) {
            for (int col = 0; col < width; col++) {
                double diff = row[col] - means[col];
                standardDeviations[col] += diff * diff;
            }
        }
        for (int col = 0; col < width; col++) {
            standardDeviations[col] = Math.sqrt(standardDeviations[col] / rows.length);
        }

        return new StandardScaler(means, standardDeviations);
    }

    public static double[][] transformStandard(double[][] rows, StandardScaler scaler) {
        int width = validateMatrix(rows);
        if (width != scaler.means().length || width != scaler.standardDeviations().length) {
            throw new IllegalArgumentException("matrix width must match scaler width");
        }

        double[][] out = new double[rows.length][width];
        for (int rowIndex = 0; rowIndex < rows.length; rowIndex++) {
            for (int col = 0; col < width; col++) {
                double standardDeviation = scaler.standardDeviations()[col];
                out[rowIndex][col] = standardDeviation == 0.0
                    ? 0.0
                    : (rows[rowIndex][col] - scaler.means()[col]) / standardDeviation;
            }
        }
        return out;
    }

    public static MinMaxScaler fitMinMaxScaler(double[][] rows) {
        int width = validateMatrix(rows);
        double[] minimums = rows[0].clone();
        double[] maximums = rows[0].clone();

        for (int rowIndex = 1; rowIndex < rows.length; rowIndex++) {
            for (int col = 0; col < width; col++) {
                minimums[col] = Math.min(minimums[col], rows[rowIndex][col]);
                maximums[col] = Math.max(maximums[col], rows[rowIndex][col]);
            }
        }

        return new MinMaxScaler(minimums, maximums);
    }

    public static double[][] transformMinMax(double[][] rows, MinMaxScaler scaler) {
        int width = validateMatrix(rows);
        if (width != scaler.minimums().length || width != scaler.maximums().length) {
            throw new IllegalArgumentException("matrix width must match scaler width");
        }

        double[][] out = new double[rows.length][width];
        for (int rowIndex = 0; rowIndex < rows.length; rowIndex++) {
            for (int col = 0; col < width; col++) {
                double span = scaler.maximums()[col] - scaler.minimums()[col];
                out[rowIndex][col] = span == 0.0 ? 0.0 : (rows[rowIndex][col] - scaler.minimums()[col]) / span;
            }
        }
        return out;
    }

    private static int validateMatrix(double[][] rows) {
        if (rows.length == 0 || rows[0].length == 0) {
            throw new IllegalArgumentException("matrix must have at least one row and one column");
        }

        int width = rows[0].length;
        for (double[] row : rows) {
            if (row.length != width) {
                throw new IllegalArgumentException("all rows must have the same number of columns");
            }
        }
        return width;
    }
}
