/// Named numeric and version constants for the ML02 gradient-descent package.
///
/// Standing Rule #20 forbids magic numbers in operational logic, so every
/// threshold the optimizer consults lives here under a descriptive name.
public enum GradientDescentConstants {
  /// The largest vector length `sgd` will accept (2^20 elements).
  ///
  /// Why a ceiling at all: the output buffer is sized from caller-supplied
  /// input, so an unbounded length would let a hostile caller force an
  /// arbitrarily large allocation (CWE-400, uncontrolled resource
  /// consumption). The check runs *before* any output is allocated.
  public static let maxVectorLength = 1_048_576

  /// The semantic version of this package.
  public static let version = "0.1.0"
}
