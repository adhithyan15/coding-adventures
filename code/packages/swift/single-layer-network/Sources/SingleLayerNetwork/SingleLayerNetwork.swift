import Foundation

public let singleLayerNetworkVersion = "0.1.0"

public enum ActivationName {
    case linear
    case sigmoid
}

public struct TrainingStep {
    public let predictions: [[Double]]
    public let errors: [[Double]]
    public let weightGradients: [[Double]]
    public let biasGradients: [Double]
    public let nextWeights: [[Double]]
    public let nextBiases: [Double]
    public let loss: Double
}

public struct SingleLayerNetwork {
    public private(set) var weights: [[Double]]
    public private(set) var biases: [Double]
    public let activation: ActivationName

    public init(inputCount: Int, outputCount: Int, activation: ActivationName = .linear) {
        self.weights = Array(repeating: Array(repeating: 0.0, count: outputCount), count: inputCount)
        self.biases = Array(repeating: 0.0, count: outputCount)
        self.activation = activation
    }

    public func predict(_ inputs: [[Double]]) throws -> [[Double]] {
        try predictWithParameters(inputs: inputs, weights: weights, biases: biases, activation: activation)
    }

    @discardableResult
    public mutating func fit(_ inputs: [[Double]], _ targets: [[Double]], learningRate: Double = 0.05, epochs: Int = 100) throws -> [TrainingStep] {
        var history: [TrainingStep] = []
        for _ in 0..<epochs {
            let step = try trainOneEpochWithMatrices(
                inputs: inputs,
                targets: targets,
                weights: weights,
                biases: biases,
                learningRate: learningRate,
                activation: activation
            )
            weights = step.nextWeights
            biases = step.nextBiases
            history.append(step)
        }
        return history
    }
}

public enum SingleLayerNetworkError: Error {
    case invalidShape(String)
}

public func predictWithParameters(inputs: [[Double]], weights: [[Double]], biases: [Double], activation: ActivationName = .linear) throws -> [[Double]] {
    let (sampleCount, inputCount) = try validateMatrix("inputs", inputs)
    let (weightRows, outputCount) = try validateMatrix("weights", weights)
    guard inputCount == weightRows else { throw SingleLayerNetworkError.invalidShape("input column count must match weight row count") }
    guard biases.count == outputCount else { throw SingleLayerNetworkError.invalidShape("bias count must match output count") }

    return (0..<sampleCount).map { row in
        (0..<outputCount).map { output in
            var total = biases[output]
            for input in 0..<inputCount {
                total += inputs[row][input] * weights[input][output]
            }
            return activate(total, activation)
        }
    }
}

public func trainOneEpochWithMatrices(
    inputs: [[Double]],
    targets: [[Double]],
    weights: [[Double]],
    biases: [Double],
    learningRate: Double,
    activation: ActivationName = .linear
) throws -> TrainingStep {
    let (sampleCount, inputCount) = try validateMatrix("inputs", inputs)
    let (targetRows, outputCount) = try validateMatrix("targets", targets)
    let (weightRows, weightCols) = try validateMatrix("weights", weights)
    guard targetRows == sampleCount else { throw SingleLayerNetworkError.invalidShape("inputs and targets must have the same row count") }
    guard weightRows == inputCount && weightCols == outputCount else { throw SingleLayerNetworkError.invalidShape("weights must be shaped input_count x output_count") }
    guard biases.count == outputCount else { throw SingleLayerNetworkError.invalidShape("bias count must match output count") }

    let predictions = try predictWithParameters(inputs: inputs, weights: weights, biases: biases, activation: activation)
    let scale = 2.0 / Double(sampleCount * outputCount)
    var errors = Array(repeating: Array(repeating: 0.0, count: outputCount), count: sampleCount)
    var deltas = Array(repeating: Array(repeating: 0.0, count: outputCount), count: sampleCount)
    var lossTotal = 0.0
    for row in 0..<sampleCount {
        for output in 0..<outputCount {
            let error = predictions[row][output] - targets[row][output]
            errors[row][output] = error
            deltas[row][output] = scale * error * derivativeFromOutput(predictions[row][output], activation)
            lossTotal += error * error
        }
    }

    var weightGradients = Array(repeating: Array(repeating: 0.0, count: outputCount), count: inputCount)
    var nextWeights = Array(repeating: Array(repeating: 0.0, count: outputCount), count: inputCount)
    for input in 0..<inputCount {
        for output in 0..<outputCount {
            for row in 0..<sampleCount {
                weightGradients[input][output] += inputs[row][input] * deltas[row][output]
            }
            nextWeights[input][output] = weights[input][output] - learningRate * weightGradients[input][output]
        }
    }

    var biasGradients = Array(repeating: 0.0, count: outputCount)
    var nextBiases = Array(repeating: 0.0, count: outputCount)
    for output in 0..<outputCount {
        for row in 0..<sampleCount {
            biasGradients[output] += deltas[row][output]
        }
        nextBiases[output] = biases[output] - learningRate * biasGradients[output]
    }

    return TrainingStep(
        predictions: predictions,
        errors: errors,
        weightGradients: weightGradients,
        biasGradients: biasGradients,
        nextWeights: nextWeights,
        nextBiases: nextBiases,
        loss: lossTotal / Double(sampleCount * outputCount)
    )
}

private func validateMatrix(_ name: String, _ matrix: [[Double]]) throws -> (Int, Int) {
    guard !matrix.isEmpty else { throw SingleLayerNetworkError.invalidShape("\(name) must contain at least one row") }
    let width = matrix[0].count
    guard width > 0 else { throw SingleLayerNetworkError.invalidShape("\(name) must contain at least one column") }
    guard matrix.allSatisfy({ $0.count == width }) else { throw SingleLayerNetworkError.invalidShape("\(name) must be rectangular") }
    return (matrix.count, width)
}

private func activate(_ value: Double, _ activation: ActivationName) -> Double {
    switch activation {
    case .linear:
        return value
    case .sigmoid:
        if value >= 0.0 {
            return 1.0 / (1.0 + exp(-value))
        }
        let z = exp(value)
        return z / (1.0 + z)
    }
}

private func derivativeFromOutput(_ output: Double, _ activation: ActivationName) -> Double {
    switch activation {
    case .linear:
        return 1.0
    case .sigmoid:
        return output * (1.0 - output)
    }
}
