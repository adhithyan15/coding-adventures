package com.codingadventures.featurenormalization;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class FeatureNormalizationTest {
    private static final double[][] ROWS = {
        {1000.0, 3.0, 1.0},
        {1500.0, 4.0, 0.0},
        {2000.0, 5.0, 1.0},
    };

    @Test
    void standardScalerCentersAndScalesColumns() {
        FeatureNormalization.StandardScaler scaler = FeatureNormalization.fitStandardScaler(ROWS);
        assertEquals(1500.0, scaler.means()[0], 1.0e-9);
        assertEquals(4.0, scaler.means()[1], 1.0e-9);

        double[][] transformed = FeatureNormalization.transformStandard(ROWS, scaler);
        assertEquals(-1.224744871391589, transformed[0][0], 1.0e-9);
        assertEquals(0.0, transformed[1][0], 1.0e-9);
        assertEquals(1.224744871391589, transformed[2][0], 1.0e-9);
    }

    @Test
    void minMaxScalerMapsColumnsToUnitRange() {
        double[][] transformed = FeatureNormalization.transformMinMax(ROWS, FeatureNormalization.fitMinMaxScaler(ROWS));
        assertArrayEquals(new double[] {0.0, 0.0, 1.0}, transformed[0], 1.0e-9);
        assertArrayEquals(new double[] {0.5, 0.5, 0.0}, transformed[1], 1.0e-9);
        assertArrayEquals(new double[] {1.0, 1.0, 1.0}, transformed[2], 1.0e-9);
    }

    @Test
    void constantColumnsMapToZero() {
        double[][] rows = {{1.0, 7.0}, {2.0, 7.0}};
        double[][] standard = FeatureNormalization.transformStandard(rows, FeatureNormalization.fitStandardScaler(rows));
        double[][] minMax = FeatureNormalization.transformMinMax(rows, FeatureNormalization.fitMinMaxScaler(rows));

        assertEquals(0.0, standard[0][1], 1.0e-9);
        assertEquals(0.0, minMax[0][1], 1.0e-9);
    }
}
