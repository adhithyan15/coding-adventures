/// Every user-visible string the gradient-descent package can emit.
///
/// Standing Rule #20: operational logic contains zero string literals. Error
/// ids, messages and placeholders are all centralised here so that they are
/// static, reviewable in one place, and provably free of caller payload.
public enum GradientDescentMessages {
  // MARK: Error ids (stable, machine-readable, kebab-case)

  /// Id for `GradientDescentErrorCode.lengthMismatch`.
  public static let lengthMismatchId = "length-mismatch"
  /// Id for `GradientDescentErrorCode.emptyInput`.
  public static let emptyInputId = "empty-input"
  /// Id for `GradientDescentErrorCode.inputTooLarge`.
  public static let inputTooLargeId = "input-too-large"
  /// Id for `GradientDescentErrorCode.nonFiniteLearningRate`.
  public static let nonFiniteLearningRateId = "non-finite-learning-rate"
  /// Id for `GradientDescentErrorCode.nonFiniteInput`.
  public static let nonFiniteInputId = "non-finite-input"
  /// Id for `GradientDescentErrorCode.numericOverflow`.
  public static let numericOverflowId = "numeric-overflow"

  // MARK: Static, payload-blind messages

  /// Message for `GradientDescentErrorCode.lengthMismatch`.
  public static let lengthMismatchMessage = "weights and gradients must have the same length"
  /// Message for `GradientDescentErrorCode.emptyInput`.
  public static let emptyInputMessage = "weights and gradients must not be empty"
  /// Message for `GradientDescentErrorCode.inputTooLarge`.
  public static let inputTooLargeMessage = "input vectors exceed the maximum supported length"
  /// Message for `GradientDescentErrorCode.nonFiniteLearningRate`.
  public static let nonFiniteLearningRateMessage = "learning rate must be a finite number"
  /// Message for `GradientDescentErrorCode.nonFiniteInput`.
  public static let nonFiniteInputMessage = "weights and gradients must be finite numbers"
  /// Message for `GradientDescentErrorCode.numericOverflow`.
  public static let numericOverflowMessage = "updated weight is not a finite number"

  // MARK: Placeholders and formatting

  /// Placeholder used wherever a sensitive value would otherwise be shown.
  public static let redactedPlaceholder = "<redacted>"
  /// Placeholder used wherever an invalid value would otherwise be shown.
  public static let invalidPlaceholder = "<invalid>"
  /// Separator between the error id and the message in `description`.
  public static let descriptionSeparator = ": "
}
