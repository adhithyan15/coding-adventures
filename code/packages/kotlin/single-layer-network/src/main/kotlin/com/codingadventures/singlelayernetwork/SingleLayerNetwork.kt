package com.codingadventures.singlelayernetwork

import kotlin.math.exp

const val VERSION = "0.1.0"

enum class ActivationName {
    LINEAR,
    SIGMOID
}

data class TrainingStep(
    val predictions: Array<DoubleArray>,
    val errors: Array<DoubleArray>,
    val weightGradients: Array<DoubleArray>,
    val biasGradients: DoubleArray,
    val nextWeights: Array<DoubleArray>,
    val nextBiases: DoubleArray,
    val loss: Double,
)

class SingleLayerNetwork(
    inputCount: Int,
    outputCount: Int,
    private val activation: ActivationName = ActivationName.LINEAR,
) {
    var weights: Array<DoubleArray> = Array(inputCount) { DoubleArray(outputCount) }
        private set
    var biases: DoubleArray = DoubleArray(outputCount)
        private set

    fun predict(inputs: Array<DoubleArray>): Array<DoubleArray> =
        predictWithParameters(inputs, weights, biases, activation)

    fun fit(inputs: Array<DoubleArray>, targets: Array<DoubleArray>, learningRate: Double = 0.05, epochs: Int = 100): List<TrainingStep> {
        val history = mutableListOf<TrainingStep>()
        repeat(epochs) {
            val step = trainOneEpochWithMatrices(inputs, targets, weights, biases, learningRate, activation)
            weights = step.nextWeights
            biases = step.nextBiases
            history.add(step)
        }
        return history
    }
}

fun predictWithParameters(
    inputs: Array<DoubleArray>,
    weights: Array<DoubleArray>,
    biases: DoubleArray,
    activation: ActivationName = ActivationName.LINEAR,
): Array<DoubleArray> {
    val (sampleCount, inputCount) = validateMatrix("inputs", inputs)
    val (weightRows, outputCount) = validateMatrix("weights", weights)
    require(inputCount == weightRows) { "input column count must match weight row count" }
    require(biases.size == outputCount) { "bias count must match output count" }

    return Array(sampleCount) { row ->
        DoubleArray(outputCount) { output ->
            var total = biases[output]
            for (input in 0 until inputCount) {
                total += inputs[row][input] * weights[input][output]
            }
            activate(total, activation)
        }
    }
}

fun trainOneEpochWithMatrices(
    inputs: Array<DoubleArray>,
    targets: Array<DoubleArray>,
    weights: Array<DoubleArray>,
    biases: DoubleArray,
    learningRate: Double,
    activation: ActivationName = ActivationName.LINEAR,
): TrainingStep {
    val (sampleCount, inputCount) = validateMatrix("inputs", inputs)
    val (targetRows, outputCount) = validateMatrix("targets", targets)
    val (weightRows, weightCols) = validateMatrix("weights", weights)
    require(targetRows == sampleCount) { "inputs and targets must have the same row count" }
    require(weightRows == inputCount && weightCols == outputCount) { "weights must be shaped input_count x output_count" }
    require(biases.size == outputCount) { "bias count must match output count" }

    val predictions = predictWithParameters(inputs, weights, biases, activation)
    val scale = 2.0 / (sampleCount * outputCount)
    val errors = Array(sampleCount) { DoubleArray(outputCount) }
    val deltas = Array(sampleCount) { DoubleArray(outputCount) }
    var lossTotal = 0.0
    for (row in 0 until sampleCount) {
        for (output in 0 until outputCount) {
            val error = predictions[row][output] - targets[row][output]
            errors[row][output] = error
            deltas[row][output] = scale * error * derivativeFromOutput(predictions[row][output], activation)
            lossTotal += error * error
        }
    }

    val weightGradients = Array(inputCount) { DoubleArray(outputCount) }
    val nextWeights = Array(inputCount) { DoubleArray(outputCount) }
    for (input in 0 until inputCount) {
        for (output in 0 until outputCount) {
            for (row in 0 until sampleCount) {
                weightGradients[input][output] += inputs[row][input] * deltas[row][output]
            }
            nextWeights[input][output] = weights[input][output] - learningRate * weightGradients[input][output]
        }
    }

    val biasGradients = DoubleArray(outputCount)
    val nextBiases = DoubleArray(outputCount)
    for (output in 0 until outputCount) {
        for (row in 0 until sampleCount) {
            biasGradients[output] += deltas[row][output]
        }
        nextBiases[output] = biases[output] - learningRate * biasGradients[output]
    }

    return TrainingStep(
        predictions = predictions,
        errors = errors,
        weightGradients = weightGradients,
        biasGradients = biasGradients,
        nextWeights = nextWeights,
        nextBiases = nextBiases,
        loss = lossTotal / (sampleCount * outputCount),
    )
}

private fun activate(value: Double, activation: ActivationName): Double =
    when (activation) {
        ActivationName.LINEAR -> value
        ActivationName.SIGMOID -> if (value >= 0.0) {
            1.0 / (1.0 + exp(-value))
        } else {
            val z = exp(value)
            z / (1.0 + z)
        }
    }

private fun derivativeFromOutput(output: Double, activation: ActivationName): Double =
    when (activation) {
        ActivationName.LINEAR -> 1.0
        ActivationName.SIGMOID -> output * (1.0 - output)
    }

private fun validateMatrix(name: String, matrix: Array<DoubleArray>): Pair<Int, Int> {
    require(matrix.isNotEmpty()) { "$name must contain at least one row" }
    val width = matrix.first().size
    require(width > 0) { "$name must contain at least one column" }
    require(matrix.all { it.size == width }) { "$name must be rectangular" }
    return matrix.size to width
}
