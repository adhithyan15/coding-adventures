/// Static, payload-blind diagnostic text for the ML02 optimizer.
///
/// Why: Standing Rule #20 keeps every error id, message and placeholder in
/// this one file so operational code carries no string literals and no
/// diagnostic can ever echo caller weights, gradients, lengths or rates.
library;

const String errorIdNullInput = 'null-input';
const String errorIdLengthMismatch = 'length-mismatch';
const String errorIdEmptyInput = 'empty-input';
const String errorIdInputTooLarge = 'input-too-large';
const String errorIdNonFiniteLearningRate = 'non-finite-learning-rate';
const String errorIdNonFiniteInput = 'non-finite-input';
const String errorIdNumericOverflow = 'numeric-overflow';

const String messageNullInput = 'weights and gradients must not be null';
const String messageLengthMismatch =
    'weights and gradients must have the same length';
const String messageEmptyInput = 'weights and gradients must not be empty';
const String messageInputTooLarge =
    'input vectors exceed the maximum supported length';
const String messageNonFiniteLearningRate =
    'learning rate must be a finite number';
const String messageNonFiniteInput =
    'weights and gradients must be finite numbers';
const String messageNumericOverflow = 'updated weight is not a finite number';

const String redactedPlaceholder = '<redacted>';
const String invalidPlaceholder = '<invalid>';
