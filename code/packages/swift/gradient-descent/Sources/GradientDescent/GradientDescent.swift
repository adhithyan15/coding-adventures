/// ML02 optimizers: pure functions that turn gradients into updated weights.
///
/// Only vanilla Stochastic Gradient Descent is implemented:
///
///     w_new[i] = w_old[i] - (learningRate * gradients[i])
///
/// See `code/specs/ML02-gradient-descent.md`.
public enum GradientDescent {
  /// Performs one SGD step and returns a fresh array of updated weights.
  ///
  /// Validation runs in a fixed precedence and fails closed; the first failing
  /// check wins:
  ///
  /// 1. `length-mismatch` — `weights.count != gradients.count`
  /// 2. `empty-input` — the vectors are empty
  /// 3. `input-too-large` — longer than `GradientDescentConstants.maxVectorLength`
  /// 4. `non-finite-learning-rate` — `learningRate` is NaN or infinite
  /// 5. `non-finite-input` — any weight or gradient is NaN or infinite
  /// 6. `numeric-overflow` — any computed step or update is not finite
  ///
  /// - Parameters:
  ///   - weights: Current parameter values.
  ///   - gradients: Partial derivatives of the loss, one per weight.
  ///   - learningRate: Step size multiplier (alpha).
  /// - Returns: A new array; neither input is modified.
  /// - Throws: `GradientDescentError` describing the first violated rule.
  public static func sgd(
    _ weights: [Double],
    _ gradients: [Double],
    learningRate: Double
  ) throws -> [Double] {
    guard weights.count == gradients.count else {
      throw GradientDescentError(.lengthMismatch)
    }
    guard !weights.isEmpty else {
      throw GradientDescentError(.emptyInput)
    }
    // CWE-400: refuse oversized inputs *before* the output buffer is
    // allocated, so the allocation size is bounded no matter what the caller
    // passes in.
    guard weights.count <= GradientDescentConstants.maxVectorLength else {
      throw GradientDescentError(.inputTooLarge)
    }
    // NaN/Inf poisoning: a single non-finite learning rate or input would
    // silently contaminate every downstream weight (NaN propagates through
    // all arithmetic). Reject it up front instead of returning garbage.
    guard learningRate.isFinite else {
      throw GradientDescentError(.nonFiniteLearningRate)
    }
    // Full pre-pass over ALL inputs before computing anything, so a
    // non-finite input is always reported as `non-finite-input` even when an
    // earlier element would have overflowed during the update.
    for index in weights.indices
    where !weights[index].isFinite || !gradients[index].isFinite {
      throw GradientDescentError(.nonFiniteInput)
    }

    // Alias safety: Swift arrays are value types with copy-on-write, and the
    // output is a brand-new buffer. `sgd(x, x, lr)` therefore reads both
    // operands from the caller's unchanged storage, and the caller's arrays
    // can never observe a write made here (or to the returned array later).
    var updated = [Double]()
    updated.reserveCapacity(weights.count)
    for index in weights.indices {
      let weight = weights[index]
      let gradient = gradients[index]
      // Unfused rounding determinism: the multiply and subtract are two
      // separately rounded IEEE-754 binary64 operations. We deliberately do
      // NOT use `addingProduct` / fma, whose single rounding would give
      // results that differ in the last bit from every other language lane.
      let step = learningRate * gradient
      let next = weight - step
      // Overflow refusal: finite inputs can still overflow (for example
      // 1e308 - (-1e308) = +Inf). Fail closed rather than return +-Inf/NaN.
      // If step overflows, next = weight - step is already non-finite.
      guard next.isFinite else {
        throw GradientDescentError(.numericOverflow)
      }
      updated.append(next)
    }
    return updated
  }

  /// Unlabeled convenience overload of `sgd(_:_:learningRate:)`.
  public static func sgd(
    _ weights: [Double],
    _ gradients: [Double],
    _ learningRate: Double
  ) throws -> [Double] {
    try sgd(weights, gradients, learningRate: learningRate)
  }
}
