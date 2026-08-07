/// Neural-network **activation functions** and their derivatives, in pure Dart.
///
/// An activation function is the non-linearity a neuron applies to its weighted
/// input. Its derivative is what backpropagation multiplies by when sending the
/// error signal backward, so every function here comes paired with its
/// derivative.
///
/// All functions operate on a single [double] (an IEEE-754 `f64`), matching the
/// `f64` reference implementation exactly. Dart's `dart:math` lacks `tanh` and
/// `log1p`, so those are implemented here in numerically stable form.
library activation_functions;

import 'dart:math' as math;

/// The slope applied to negative inputs by [leakyRelu] (a small positive
/// constant that keeps "dead" neurons learning).
const double leakyReluSlope = 0.01;

// ─── Linear (identity) ───────────────────────────────────────────────────────

/// Identity activation: returns [x] unchanged. Used for regression outputs.
double linear(double x) => x;

/// Derivative of [linear] — always 1.
double linearDerivative(double x) => 1.0;

// ─── Sigmoid (logistic) ──────────────────────────────────────────────────────

/// Logistic sigmoid `1 / (1 + e^-x)`, squashing any input into `(0, 1)`.
///
/// Guards against overflow of `e^-x`: for `x < -709` the result underflows to 0
/// and for `x > 709` it saturates to 1 (709 is near the largest `x` for which
/// `e^x` is finite in `f64`).
double sigmoid(double x) {
  if (x < -709.0) return 0.0;
  if (x > 709.0) return 1.0;
  return 1.0 / (1.0 + math.exp(-x));
}

/// Derivative of [sigmoid] — `σ(x)·(1 − σ(x))`, peaking at 0.25 when `x = 0`.
double sigmoidDerivative(double x) {
  final s = sigmoid(x);
  return s * (1.0 - s);
}

// ─── ReLU (rectified linear unit) ────────────────────────────────────────────

/// ReLU `max(0, x)` — the workhorse activation of deep networks.
double relu(double x) => x > 0.0 ? x : 0.0;

/// Derivative of [relu] — 1 for positive input, 0 otherwise (0 at `x = 0`).
double reluDerivative(double x) => x > 0.0 ? 1.0 : 0.0;

// ─── Leaky ReLU ──────────────────────────────────────────────────────────────

/// Leaky ReLU — like [relu] but with a small [leakyReluSlope] gradient for
/// negative inputs, so neurons never fully die.
double leakyRelu(double x) => x > 0.0 ? x : leakyReluSlope * x;

/// Derivative of [leakyRelu] — 1 for positive input, [leakyReluSlope] otherwise.
double leakyReluDerivative(double x) => x > 0.0 ? 1.0 : leakyReluSlope;

// ─── Tanh (hyperbolic tangent) ───────────────────────────────────────────────

/// Hyperbolic tangent, squashing any input into `(−1, 1)`.
///
/// `dart:math` has no `tanh`, so this uses the numerically stable identity
/// `tanh(x) = (e^{2x} − 1) / (e^{2x} + 1)` and saturates beyond `|x| = 20`,
/// where `tanh` is already within one `f64` ulp of `±1`.
double tanh(double x) {
  if (x > 20.0) return 1.0;
  if (x < -20.0) return -1.0;
  final e2x = math.exp(2.0 * x);
  return (e2x - 1.0) / (e2x + 1.0);
}

/// Derivative of [tanh] — `1 − tanh(x)²`.
double tanhDerivative(double x) {
  final t = tanh(x);
  return 1.0 - t * t;
}

// ─── Softplus ────────────────────────────────────────────────────────────────

/// Softplus `ln(1 + e^x)` — a smooth approximation of [relu].
///
/// Computed in the numerically stable form
/// `ln(1 + e^{−|x|}) + max(x, 0)`, which avoids overflow of `e^x` for large
/// positive `x`. Uses the local [_ln1p] to keep precision when `e^{−|x|}` is
/// tiny.
double softplus(double x) => _ln1p(math.exp(-x.abs())) + math.max(x, 0.0);

/// Derivative of [softplus] — exactly the [sigmoid].
double softplusDerivative(double x) => sigmoid(x);

/// `ln(1 + v)` computed accurately for small `v`, mirroring Rust's `f64::ln_1p`.
///
/// The naive `ln(1 + v)` loses precision when `v` is tiny (`1 + v` rounds to
/// 1). This uses the standard correction based on `w = 1 + v`: when `w` differs
/// from 1, it multiplies `ln(w)` by `v / (w − 1)` to recover the lost bits.
double _ln1p(double v) {
  final w = 1.0 + v;
  if (w == 1.0) return v; // v so small that 1 + v is exactly 1
  return math.log(w) * (v / (w - 1.0));
}
