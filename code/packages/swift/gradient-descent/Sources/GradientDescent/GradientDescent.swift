public struct GradientDescent {
    public struct MismatchedLengthError: Error {}
    public static func sgd(weights: [Double], gradients: [Double], learningRate: Double) throws -> [Double] {
        guard weights.count == gradients.count, !weights.isEmpty else {
            throw MismatchedLengthError()
        }
        var result = [Double](repeating: 0.0, count: weights.count)
        for i in 0..<weights.count {
            result[i] = weights[i] - (learningRate * gradients[i])
        }
        return result
    }
}
