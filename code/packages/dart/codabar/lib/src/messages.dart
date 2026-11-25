import 'errors.dart';

/// Diagnostic and error messages for Codabar encoding in Dart, strictly
/// payload-blind: callers branch on the stable identifier, never the text.
const String codabarErrorInputTooLong = 'input-too-long';
const String codabarErrorInvalidCharacter = 'invalid-character';
const String codabarErrorInvalidGuard = 'invalid-guard';
const String codabarErrorGuardConflict = 'guard-conflict';

const String codabarMessageInputTooLong =
    'Codabar input must contain at most 4096 characters';
const String codabarMessageInvalidCharacter =
    'Codabar input contains a character outside its alphabet';
const String codabarMessageInvalidGuard =
    'Codabar guards must be A, B, C, or D on both edges or neither';
// NOTE: the reference renders offending characters into its messages
// (format!/Debug) and has no guard-conflict error at all (it silently
// ignores disagreeing options on guarded input). The v1 contract
// requires payload-blind identifiers plus the conflict check, so the
// Dart messages carry no input bytes and the conflict is reported.
const String codabarMessageGuardConflict =
    'Codabar guard options disagree with the guarded input';
const String codabarMessageInvalidScalars =
    'Codabar input must contain valid Unicode scalar values';
const String codabarMessageNullInput = 'Codabar input must not be null';

String codabarMessageFor(CodabarErrorCode? code) {
  switch (code) {
    case CodabarErrorCode.inputTooLong:
      return codabarMessageInputTooLong;
    case CodabarErrorCode.invalidCharacter:
    case null:
      return codabarMessageInvalidCharacter;
    case CodabarErrorCode.invalidGuard:
      return codabarMessageInvalidGuard;
    case CodabarErrorCode.guardConflict:
      return codabarMessageGuardConflict;
  }
}
