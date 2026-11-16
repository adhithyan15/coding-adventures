/// The closed set of reasons `GradientDescent.sgd` can refuse an update.
///
/// Cases are declared in validation-precedence order: when several problems
/// are present at once, the earliest case listed here is the one reported.
/// (Swift's non-optional arrays make a `null-input` case unrepresentable, so
/// precedence starts at `lengthMismatch`.)
public enum GradientDescentErrorCode: CaseIterable, Equatable, Hashable, Sendable {
  /// `weights.count != gradients.count`.
  case lengthMismatch
  /// Both vectors are empty.
  case emptyInput
  /// The vectors exceed `GradientDescentConstants.maxVectorLength` (CWE-400).
  case inputTooLarge
  /// The learning rate is NaN, +Inf or -Inf.
  case nonFiniteLearningRate
  /// At least one weight or gradient is NaN, +Inf or -Inf.
  case nonFiniteInput
  /// A computed step or updated weight overflowed to a non-finite value.
  case numericOverflow

  /// Stable kebab-case identifier, resolved from `GradientDescentMessages`.
  public var errorId: String {
    switch self {
    case .lengthMismatch: return GradientDescentMessages.lengthMismatchId
    case .emptyInput: return GradientDescentMessages.emptyInputId
    case .inputTooLarge: return GradientDescentMessages.inputTooLargeId
    case .nonFiniteLearningRate: return GradientDescentMessages.nonFiniteLearningRateId
    case .nonFiniteInput: return GradientDescentMessages.nonFiniteInputId
    case .numericOverflow: return GradientDescentMessages.numericOverflowId
    }
  }

  /// Static human-readable message, resolved from `GradientDescentMessages`.
  public var message: String {
    switch self {
    case .lengthMismatch: return GradientDescentMessages.lengthMismatchMessage
    case .emptyInput: return GradientDescentMessages.emptyInputMessage
    case .inputTooLarge: return GradientDescentMessages.inputTooLargeMessage
    case .nonFiniteLearningRate: return GradientDescentMessages.nonFiniteLearningRateMessage
    case .nonFiniteInput: return GradientDescentMessages.nonFiniteInputMessage
    case .numericOverflow: return GradientDescentMessages.numericOverflowMessage
    }
  }
}

/// The only error type thrown by the gradient-descent package.
///
/// Payload-blind by construction (CWE-209): the sole stored property is the
/// closed error code, and the public initializer accepts nothing else. No
/// length, index, weight, gradient or learning rate can ever reach the
/// message, the description, or any field.
public struct GradientDescentError: Error, Equatable, Hashable, Sendable, CustomStringConvertible {
  /// The closed reason for the failure.
  public let code: GradientDescentErrorCode

  /// Creates an error for `code`. The message is always the static one from
  /// `GradientDescentMessages`; callers cannot supply their own text.
  public init(_ code: GradientDescentErrorCode) {
    self.code = code
  }

  /// Stable kebab-case identifier of the failure.
  public var errorId: String { code.errorId }

  /// Static human-readable message of the failure.
  public var message: String { code.message }

  /// `"<error-id>: <message>"`, built only from static strings.
  public var description: String {
    errorId + GradientDescentMessages.descriptionSeparator + message
  }
}
