import Foundation

public let twoLayerNetworkVersion = "0.1.0"

public enum ActivationName {
    case linear
    case sigmoid
}

public struct Parameters {
    public let inputToHiddenWeights: [[Double]]
    public let hiddenBiases: [Double]
    public let hiddenToOutputWeights: [[Double]]
    public let outputBiases: [Double]

    public init(inputToHiddenWeights: [[Double]], hiddenBiases: [Double], hiddenToOutputWeights: [[Double]], outputBiases: [Double]) {
        self.inputToHiddenWeights = inputToHiddenWeights
        self.hiddenBiases = hiddenBiases
        self.hiddenToOutputWeights = hiddenToOutputWeights
        self.outputBiases = outputBiases
    }
}

public struct ForwardPass {
    public let hiddenRaw: [[Double]]
    public let hiddenActivations: [[Double]]
    public let outputRaw: [[Double]]
    public let predictions: [[Double]]
}

public struct TrainingStep {
    public let predictions: [[Double]]
    public let errors: [[Double]]
    public let outputDeltas: [[Double]]
    public let hiddenDeltas: [[Double]]
    public let hiddenToOutputWeightGradients: [[Double]]
    public let outputBiasGradients: [Double]
    public let inputToHiddenWeightGradients: [[Double]]
    public let hiddenBiasGradients: [Double]
    public let nextParameters: Parameters
    public let loss: Double
}

public enum TwoLayerNetworkError: Error {
    case invalidShape(String)
}

public func xorWarmStartParameters() -> Parameters {
    Parameters(
        inputToHiddenWeights: [[4.0, -4.0], [4.0, -4.0]],
        hiddenBiases: [-2.0, 6.0],
        hiddenToOutputWeights: [[4.0], [4.0]],
        outputBiases: [-6.0]
    )
}

public func forward(
    inputs: [[Double]],
    parameters: Parameters,
    hiddenActivation: ActivationName = .sigmoid,
    outputActivation: ActivationName = .sigmoid
) throws -> ForwardPass {
    let hiddenRaw = try addBiases(dot(inputs, parameters.inputToHiddenWeights), parameters.hiddenBiases)
    let hiddenActivations = hiddenRaw.map { row in row.map { activate($0, hiddenActivation) } }
    let outputRaw = try addBiases(dot(hiddenActivations, parameters.hiddenToOutputWeights), parameters.outputBiases)
    let predictions = outputRaw.map { row in row.map { activate($0, outputActivation) } }
    return ForwardPass(hiddenRaw: hiddenRaw, hiddenActivations: hiddenActivations, outputRaw: outputRaw, predictions: predictions)
}

public func trainOneEpoch(
    inputs: [[Double]],
    targets: [[Double]],
    parameters: Parameters,
    learningRate: Double,
    hiddenActivation: ActivationName = .sigmoid,
    outputActivation: ActivationName = .sigmoid
) throws -> TrainingStep {
    let (sampleCount, _) = try validateMatrix("inputs", inputs)
    let (_, outputCount) = try validateMatrix("targets", targets)
    let pass = try forward(inputs: inputs, parameters: parameters, hiddenActivation: hiddenActivation, outputActivation: outputActivation)
    let scale = 2.0 / Double(sampleCount * outputCount)
    var errors = Array(repeating: Array(repeating: 0.0, count: outputCount), count: sampleCount)
    var outputDeltas = Array(repeating: Array(repeating: 0.0, count: outputCount), count: sampleCount)
    for row in 0..<sampleCount {
        for output in 0..<outputCount {
            let error = pass.predictions[row][output] - targets[row][output]
            errors[row][output] = error
            outputDeltas[row][output] = scale * error * derivative(pass.outputRaw[row][output], pass.predictions[row][output], outputActivation)
        }
    }
    let h2oGradients = try dot(transpose(pass.hiddenActivations), outputDeltas)
    let outputBiasGradients = try columnSums(outputDeltas)
    let hiddenErrors = try dot(outputDeltas, transpose(parameters.hiddenToOutputWeights))
    let hiddenWidth = parameters.hiddenBiases.count
    var hiddenDeltas = Array(repeating: Array(repeating: 0.0, count: hiddenWidth), count: sampleCount)
    for row in 0..<sampleCount {
        for hidden in 0..<hiddenWidth {
            hiddenDeltas[row][hidden] = hiddenErrors[row][hidden] *
                derivative(pass.hiddenRaw[row][hidden], pass.hiddenActivations[row][hidden], hiddenActivation)
        }
    }
    let i2hGradients = try dot(transpose(inputs), hiddenDeltas)
    let hiddenBiasGradients = try columnSums(hiddenDeltas)
    return TrainingStep(
        predictions: pass.predictions,
        errors: errors,
        outputDeltas: outputDeltas,
        hiddenDeltas: hiddenDeltas,
        hiddenToOutputWeightGradients: h2oGradients,
        outputBiasGradients: outputBiasGradients,
        inputToHiddenWeightGradients: i2hGradients,
        hiddenBiasGradients: hiddenBiasGradients,
        nextParameters: Parameters(
            inputToHiddenWeights: subtractScaled(parameters.inputToHiddenWeights, i2hGradients, learningRate),
            hiddenBiases: subtractScaled(parameters.hiddenBiases, hiddenBiasGradients, learningRate),
            hiddenToOutputWeights: subtractScaled(parameters.hiddenToOutputWeights, h2oGradients, learningRate),
            outputBiases: subtractScaled(parameters.outputBiases, outputBiasGradients, learningRate)
        ),
        loss: meanSquaredError(errors)
    )
}

private func validateMatrix(_ name: String, _ matrix: [[Double]]) throws -> (Int, Int) {
    guard !matrix.isEmpty else { throw TwoLayerNetworkError.invalidShape("\(name) must contain at least one row") }
    let width = matrix[0].count
    guard width > 0 else { throw TwoLayerNetworkError.invalidShape("\(name) must contain at least one column") }
    guard matrix.allSatisfy({ $0.count == width }) else { throw TwoLayerNetworkError.invalidShape("\(name) must be rectangular") }
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

private func derivative(_ raw: Double, _ activated: Double, _ activation: ActivationName) -> Double {
    switch activation {
    case .linear:
        return 1.0
    case .sigmoid:
        return activated * (1.0 - activated)
    }
}

private func dot(_ left: [[Double]], _ right: [[Double]]) throws -> [[Double]] {
    let (rows, width) = try validateMatrix("left", left)
    let (rightRows, cols) = try validateMatrix("right", right)
    guard width == rightRows else { throw TwoLayerNetworkError.invalidShape("matrix shapes do not align") }
    return (0..<rows).map { row in
        (0..<cols).map { col in
            (0..<width).reduce(0.0) { sum, k in sum + left[row][k] * right[k][col] }
        }
    }
}

private func transpose(_ matrix: [[Double]]) -> [[Double]] {
    let rows = matrix.count
    let cols = matrix[0].count
    return (0..<cols).map { col in (0..<rows).map { row in matrix[row][col] } }
}

private func addBiases(_ matrix: [[Double]], _ biases: [Double]) throws -> [[Double]] {
    matrix.map { row in row.enumerated().map { col, value in value + biases[col] } }
}

private func columnSums(_ matrix: [[Double]]) throws -> [Double] {
    let (_, cols) = try validateMatrix("matrix", matrix)
    return (0..<cols).map { col in matrix.reduce(0.0) { sum, row in sum + row[col] } }
}

private func subtractScaled(_ matrix: [[Double]], _ gradients: [[Double]], _ learningRate: Double) -> [[Double]] {
    matrix.enumerated().map { rowIndex, row in
        row.enumerated().map { col, value in value - learningRate * gradients[rowIndex][col] }
    }
}

private func subtractScaled(_ values: [Double], _ gradients: [Double], _ learningRate: Double) -> [Double] {
    values.enumerated().map { index, value in value - learningRate * gradients[index] }
}

private func meanSquaredError(_ errors: [[Double]]) -> Double {
    let values = errors.flatMap { $0 }
    return values.reduce(0.0) { sum, value in sum + value * value } / Double(values.count)
}
