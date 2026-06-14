package com.codingadventures.twolayernetwork

import kotlin.math.exp

const val VERSION = "0.1.0"

enum class ActivationName {
    LINEAR,
    SIGMOID
}

data class Parameters(
    val inputToHiddenWeights: Array<DoubleArray>,
    val hiddenBiases: DoubleArray,
    val hiddenToOutputWeights: Array<DoubleArray>,
    val outputBiases: DoubleArray,
)

data class ForwardPass(
    val hiddenRaw: Array<DoubleArray>,
    val hiddenActivations: Array<DoubleArray>,
    val outputRaw: Array<DoubleArray>,
    val predictions: Array<DoubleArray>,
)

data class TrainingStep(
    val predictions: Array<DoubleArray>,
    val errors: Array<DoubleArray>,
    val outputDeltas: Array<DoubleArray>,
    val hiddenDeltas: Array<DoubleArray>,
    val hiddenToOutputWeightGradients: Array<DoubleArray>,
    val outputBiasGradients: DoubleArray,
    val inputToHiddenWeightGradients: Array<DoubleArray>,
    val hiddenBiasGradients: DoubleArray,
    val nextParameters: Parameters,
    val loss: Double,
)

fun xorWarmStartParameters(): Parameters =
    Parameters(
        inputToHiddenWeights = arrayOf(doubleArrayOf(4.0, -4.0), doubleArrayOf(4.0, -4.0)),
        hiddenBiases = doubleArrayOf(-2.0, 6.0),
        hiddenToOutputWeights = arrayOf(doubleArrayOf(4.0), doubleArrayOf(4.0)),
        outputBiases = doubleArrayOf(-6.0),
    )

fun forward(
    inputs: Array<DoubleArray>,
    parameters: Parameters,
    hiddenActivation: ActivationName = ActivationName.SIGMOID,
    outputActivation: ActivationName = ActivationName.SIGMOID,
): ForwardPass {
    val hiddenRaw = addBiases(dot(inputs, parameters.inputToHiddenWeights), parameters.hiddenBiases)
    val hiddenActivations = applyActivation(hiddenRaw, hiddenActivation)
    val outputRaw = addBiases(dot(hiddenActivations, parameters.hiddenToOutputWeights), parameters.outputBiases)
    val predictions = applyActivation(outputRaw, outputActivation)
    return ForwardPass(hiddenRaw, hiddenActivations, outputRaw, predictions)
}

fun trainOneEpoch(
    inputs: Array<DoubleArray>,
    targets: Array<DoubleArray>,
    parameters: Parameters,
    learningRate: Double,
    hiddenActivation: ActivationName = ActivationName.SIGMOID,
    outputActivation: ActivationName = ActivationName.SIGMOID,
): TrainingStep {
    val sampleCount = validateMatrix("inputs", inputs).first
    val outputCount = validateMatrix("targets", targets).second
    val pass = forward(inputs, parameters, hiddenActivation, outputActivation)
    val scale = 2.0 / (sampleCount * outputCount)
    val errors = Array(sampleCount) { DoubleArray(outputCount) }
    val outputDeltas = Array(sampleCount) { DoubleArray(outputCount) }
    for (row in 0 until sampleCount) {
        for (output in 0 until outputCount) {
            val error = pass.predictions[row][output] - targets[row][output]
            errors[row][output] = error
            outputDeltas[row][output] = scale * error * derivative(pass.outputRaw[row][output], pass.predictions[row][output], outputActivation)
        }
    }
    val h2oGradients = dot(transpose(pass.hiddenActivations), outputDeltas)
    val outputBiasGradients = columnSums(outputDeltas)
    val hiddenErrors = dot(outputDeltas, transpose(parameters.hiddenToOutputWeights))
    val hiddenWidth = parameters.hiddenBiases.size
    val hiddenDeltas = Array(sampleCount) { DoubleArray(hiddenWidth) }
    for (row in 0 until sampleCount) {
        for (hidden in 0 until hiddenWidth) {
            hiddenDeltas[row][hidden] = hiddenErrors[row][hidden] *
                derivative(pass.hiddenRaw[row][hidden], pass.hiddenActivations[row][hidden], hiddenActivation)
        }
    }
    val i2hGradients = dot(transpose(inputs), hiddenDeltas)
    val hiddenBiasGradients = columnSums(hiddenDeltas)
    return TrainingStep(
        predictions = pass.predictions,
        errors = errors,
        outputDeltas = outputDeltas,
        hiddenDeltas = hiddenDeltas,
        hiddenToOutputWeightGradients = h2oGradients,
        outputBiasGradients = outputBiasGradients,
        inputToHiddenWeightGradients = i2hGradients,
        hiddenBiasGradients = hiddenBiasGradients,
        nextParameters = Parameters(
            inputToHiddenWeights = subtractScaled(parameters.inputToHiddenWeights, i2hGradients, learningRate),
            hiddenBiases = subtractScaled(parameters.hiddenBiases, hiddenBiasGradients, learningRate),
            hiddenToOutputWeights = subtractScaled(parameters.hiddenToOutputWeights, h2oGradients, learningRate),
            outputBiases = subtractScaled(parameters.outputBiases, outputBiasGradients, learningRate),
        ),
        loss = meanSquaredError(errors),
    )
}

private fun activate(value: Double, activation: ActivationName): Double =
    when (activation) {
        ActivationName.LINEAR -> value
        ActivationName.SIGMOID -> if (value >= 0.0) 1.0 / (1.0 + exp(-value)) else {
            val z = exp(value)
            z / (1.0 + z)
        }
    }

private fun derivative(raw: Double, activated: Double, activation: ActivationName): Double =
    when (activation) {
        ActivationName.LINEAR -> 1.0
        ActivationName.SIGMOID -> activated * (1.0 - activated)
    }

private fun dot(left: Array<DoubleArray>, right: Array<DoubleArray>): Array<DoubleArray> {
    val (rows, width) = validateMatrix("left", left)
    val (rightRows, cols) = validateMatrix("right", right)
    require(width == rightRows) { "matrix shapes do not align" }
    return Array(rows) { row ->
        DoubleArray(cols) { col -> (0 until width).sumOf { k -> left[row][k] * right[k][col] } }
    }
}

private fun transpose(matrix: Array<DoubleArray>): Array<DoubleArray> {
    val (rows, cols) = validateMatrix("matrix", matrix)
    return Array(cols) { col -> DoubleArray(rows) { row -> matrix[row][col] } }
}

private fun addBiases(matrix: Array<DoubleArray>, biases: DoubleArray): Array<DoubleArray> =
    Array(matrix.size) { row -> DoubleArray(matrix[row].size) { col -> matrix[row][col] + biases[col] } }

private fun applyActivation(matrix: Array<DoubleArray>, activation: ActivationName): Array<DoubleArray> =
    Array(matrix.size) { row -> DoubleArray(matrix[row].size) { col -> activate(matrix[row][col], activation) } }

private fun columnSums(matrix: Array<DoubleArray>): DoubleArray {
    val cols = validateMatrix("matrix", matrix).second
    return DoubleArray(cols) { col -> matrix.sumOf { row -> row[col] } }
}

private fun subtractScaled(matrix: Array<DoubleArray>, gradients: Array<DoubleArray>, learningRate: Double): Array<DoubleArray> =
    Array(matrix.size) { row -> DoubleArray(matrix[row].size) { col -> matrix[row][col] - learningRate * gradients[row][col] } }

private fun subtractScaled(values: DoubleArray, gradients: DoubleArray, learningRate: Double): DoubleArray =
    DoubleArray(values.size) { index -> values[index] - learningRate * gradients[index] }

private fun meanSquaredError(errors: Array<DoubleArray>): Double =
    errors.flatMap { row -> row.asIterable() }.sumOf { value -> value * value } / errors.sumOf { it.size }

private fun validateMatrix(name: String, matrix: Array<DoubleArray>): Pair<Int, Int> {
    require(matrix.isNotEmpty()) { "$name must contain at least one row" }
    val width = matrix.first().size
    require(width > 0) { "$name must contain at least one column" }
    require(matrix.all { it.size == width }) { "$name must be rectangular" }
    return matrix.size to width
}
