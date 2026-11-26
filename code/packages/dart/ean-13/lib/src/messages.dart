import 'errors.dart';

/// Diagnostic and error messages for EAN-13 encoding in Dart, strictly
/// payload-blind: callers branch on the stable identifier, never the text.
const String ean13ErrorInputTooLong = 'input-too-long';
const String ean13ErrorInvalidLength = 'invalid-length';
const String ean13ErrorInvalidCharacter = 'invalid-character';
const String ean13ErrorInvalidCheckDigit = 'invalid-check-digit';

const String ean13MessageInputTooLong =
    'EAN-13 input must contain at most 4096 characters';
const String ean13MessageInvalidLength =
    'EAN-13 input must contain 12 digits or 13 digits';
const String ean13MessageInvalidCharacter =
    'EAN-13 input must contain digits only';
const String ean13MessageInvalidCheckDigit =
    'EAN-13 check digit does not match the payload';
// NOTE: the reference renders the received digit into its check-digit
// message ("expected {} but received {}"). The v1 contract requires
// payload-blind identifiers, so the Dart message carries no input bytes;
// the stable `invalid-check-digit` identifier is the branch signal.
const String ean13MessageInvalidScalars =
    'EAN-13 input must contain valid Unicode scalar values';
const String ean13MessageNullInput = 'EAN-13 input must not be null';

String ean13MessageFor(Ean13ErrorCode? code) {
  switch (code) {
    case Ean13ErrorCode.inputTooLong:
      return ean13MessageInputTooLong;
    case Ean13ErrorCode.invalidLength:
      return ean13MessageInvalidLength;
    case Ean13ErrorCode.invalidCharacter:
    case null:
      return ean13MessageInvalidCharacter;
    case Ean13ErrorCode.invalidCheckDigit:
      return ean13MessageInvalidCheckDigit;
  }
}
