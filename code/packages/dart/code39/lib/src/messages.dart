import 'errors.dart';

/// Diagnostic and error messages for Code 39 encoding in Dart, strictly
/// payload-blind: callers branch on the stable identifier, never the text.
const String code39ErrorInputTooLong = 'input-too-long';
const String code39ErrorInvalidCharacter = 'invalid-character';
const String code39ErrorReservedCharacter = 'reserved-character';

const String code39MessageInputTooLong =
    'Code 39 input must contain at most 4096 characters';
const String code39MessageInvalidCharacter =
    'Code 39 input contains a character outside its alphabet';
// NOTE: the reference renders the offending character into both of its
// messages (format!/Debug). The v1 contract requires payload-blind
// identifiers, so the Dart messages carry no input bytes; the stable
// identifier is the branch signal.
const String code39MessageReservedCharacter =
    'Code 39 input must not contain "*" because it is reserved for start/stop';
const String code39MessageInvalidScalars =
    'Code 39 input must contain valid Unicode scalar values';
const String code39MessageNullInput = 'Code 39 input must not be null';

String code39MessageFor(Code39ErrorCode? code) {
  switch (code) {
    case Code39ErrorCode.inputTooLong:
      return code39MessageInputTooLong;
    case Code39ErrorCode.invalidCharacter:
    case null:
      return code39MessageInvalidCharacter;
    case Code39ErrorCode.reservedCharacter:
      return code39MessageReservedCharacter;
  }
}
