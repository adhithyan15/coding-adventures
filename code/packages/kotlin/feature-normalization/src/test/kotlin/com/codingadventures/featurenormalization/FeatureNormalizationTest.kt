package com.codingadventures.featurenormalization

import kotlin.test.Test
import kotlin.test.assertEquals

class FeatureNormalizationTest {
    private val rows = listOf(
        listOf(1000.0, 3.0, 1.0),
        listOf(1500.0, 4.0, 0.0),
        listOf(2000.0, 5.0, 1.0),
    )

    @Test
    fun standardScalerCentersAndScalesColumns() {
        val scaler = FeatureNormalization.fitStandardScaler(rows)
        assertEquals(1500.0, scaler.means[0], 1.0e-9)
        assertEquals(4.0, scaler.means[1], 1.0e-9)

        val transformed = FeatureNormalization.transformStandard(rows, scaler)
        assertEquals(-1.224744871391589, transformed[0][0], 1.0e-9)
        assertEquals(0.0, transformed[1][0], 1.0e-9)
        assertEquals(1.224744871391589, transformed[2][0], 1.0e-9)
    }

    @Test
    fun minMaxScalerMapsColumnsToUnitRange() {
        val transformed = FeatureNormalization.transformMinMax(rows, FeatureNormalization.fitMinMaxScaler(rows))

        assertEquals(listOf(0.0, 0.0, 1.0), transformed[0])
        assertEquals(listOf(0.5, 0.5, 0.0), transformed[1])
        assertEquals(listOf(1.0, 1.0, 1.0), transformed[2])
    }

    @Test
    fun constantColumnsMapToZero() {
        val data = listOf(listOf(1.0, 7.0), listOf(2.0, 7.0))

        val standard = FeatureNormalization.transformStandard(data, FeatureNormalization.fitStandardScaler(data))
        val minMax = FeatureNormalization.transformMinMax(data, FeatureNormalization.fitMinMaxScaler(data))

        assertEquals(0.0, standard[0][1], 1.0e-9)
        assertEquals(0.0, minMax[0][1], 1.0e-9)
    }
}
