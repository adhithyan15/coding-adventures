import 'constants.dart';
import 'errors.dart';

/// One step of vanilla stochastic gradient descent (ML02 SGD).
///
/// Returns a new, unmodifiable list where
/// `result[i] = weights[i] - (learningRate * gradients[i])`.
///
/// Validation runs in a fixed order and the first failure wins:
/// `length-mismatch`, `empty-input`, `input-too-large`,
/// `non-finite-learning-rate`, `non-finite-input`, `numeric-overflow`.
/// Each failure throws [InvalidGradientDescentInputError].
List<double> sgd(
  List<double> weights,
  List<double> gradients,
  double learningRate,
) {
  final length = weights.length;
  if (length != gradients.length) {
    throw InvalidGradientDescentInputError(
      GradientDescentErrorCode.lengthMismatch,
    );
  }
  if (length == 0) {
    throw InvalidGradientDescentInputError(GradientDescentErrorCode.emptyInput);
  }
  // Why: reject oversized inputs before allocating the output buffer, so
  // memory use stays bounded no matter what the caller sends (CWE-400).
  if (length > maxVectorLength) {
    throw InvalidGradientDescentInputError(
      GradientDescentErrorCode.inputTooLarge,
    );
  }
  if (!learningRate.isFinite) {
    throw InvalidGradientDescentInputError(
      GradientDescentErrorCode.nonFiniteLearningRate,
    );
  }
  // Why: one full pre-pass rejects NaN/Infinity anywhere before any
  // arithmetic, so a single poisoned gradient can never silently spread
  // NaN through a model's weights (CWE-682).
  for (var i = 0; i < length; i++) {
    if (!weights[i].isFinite || !gradients[i].isFinite) {
      throw InvalidGradientDescentInputError(
        GradientDescentErrorCode.nonFiniteInput,
      );
    }
  }

  // Why: the output is a fresh buffer and each element reads both inputs
  // before writing, so `sgd(x, x, lr)` (aliased inputs) is correct and the
  // caller's lists are never mutated.
  final updated = List<double>.filled(length, 0.0);
  for (var i = 0; i < length; i++) {
    // Why: two separate IEEE-754 binary64 operations (multiply, then
    // subtract) rather than a fused multiply-add. Each step rounds once, so
    // every language lane produces bit-identical results.
    final step = learningRate * gradients[i];
    final next = weights[i] - step;
    // Why: finite inputs can still overflow (for example
    // 1e308 - (-1e308)). Fail closed instead of returning +/-Infinity.
    // An infinite `step` always makes `next` non-finite too, so checking
    // `next` alone covers both cases.
    if (!next.isFinite) {
      throw InvalidGradientDescentInputError(
        GradientDescentErrorCode.numericOverflow,
      );
    }
    updated[i] = next;
  }
  return List<double>.unmodifiable(updated);
}
