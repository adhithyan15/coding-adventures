package com.codingadventures.featurenormalization

import kotlin.math.sqrt

data class StandardScaler(val means: List<Double>, val standardDeviations: List<Double>)

data class MinMaxScaler(val minimums: List<Double>, val maximums: List<Double>)

object FeatureNormalization {
    fun fitStandardScaler(rows: List<List<Double>>): StandardScaler {
        val width = validateMatrix(rows)
        val means = List(width) { col -> rows.sumOf { it[col] } / rows.size }
        val standardDeviations = List(width) { col ->
            sqrt(rows.sumOf {
                val diff = it[col] - means[col]
                diff * diff
            } / rows.size)
        }
        return StandardScaler(means, standardDeviations)
    }

    fun transformStandard(rows: List<List<Double>>, scaler: StandardScaler): List<List<Double>> {
        val width = validateMatrix(rows)
        require(width == scaler.means.size && width == scaler.standardDeviations.size) {
            "matrix width must match scaler width"
        }

        return rows.map { row ->
            row.mapIndexed { col, value ->
                val standardDeviation = scaler.standardDeviations[col]
                if (standardDeviation == 0.0) 0.0 else (value - scaler.means[col]) / standardDeviation
            }
        }
    }

    fun fitMinMaxScaler(rows: List<List<Double>>): MinMaxScaler {
        val width = validateMatrix(rows)
        return MinMaxScaler(
            minimums = List(width) { col -> rows.minOf { it[col] } },
            maximums = List(width) { col -> rows.maxOf { it[col] } },
        )
    }

    fun transformMinMax(rows: List<List<Double>>, scaler: MinMaxScaler): List<List<Double>> {
        val width = validateMatrix(rows)
        require(width == scaler.minimums.size && width == scaler.maximums.size) {
            "matrix width must match scaler width"
        }

        return rows.map { row ->
            row.mapIndexed { col, value ->
                val span = scaler.maximums[col] - scaler.minimums[col]
                if (span == 0.0) 0.0 else (value - scaler.minimums[col]) / span
            }
        }
    }

    private fun validateMatrix(rows: List<List<Double>>): Int {
        require(rows.isNotEmpty() && rows.first().isNotEmpty()) {
            "matrix must have at least one row and one column"
        }

        val width = rows.first().size
        require(rows.all { it.size == width }) {
            "all rows must have the same number of columns"
        }
        return width
    }
}
