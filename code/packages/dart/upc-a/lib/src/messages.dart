import 'errors.dart';

/// Diagnostic and error messages for UPC-A encoding in Dart, strictly
/// payload-blind: callers branch on the stable identifier, never the text.
const String upcAErrorInputTooLong = 'input-too-long';
const String upcAErrorInvalidLength = 'invalid-length';
const String upcAErrorInvalidCharacter = 'invalid-character';
const String upcAErrorInvalidCheckDigit = 'invalid-check-digit';

const String upcAMessageInputTooLong =
    'UPC-A input must contain at most 4096 characters';
const String upcAMessageInvalidLength =
    'UPC-A input must contain 11 digits or 12 digits';
const String upcAMessageInvalidCharacter =
    'UPC-A input must contain digits only';
const String upcAMessageInvalidCheckDigit =
    'UPC-A check digit does not match the payload';
// NOTE: the reference renders the received digit into its check-digit
// message ("expected {} but received {}"). The v1 contract requires
// payload-blind identifiers, so the Dart message carries no input bytes;
// the stable `invalid-check-digit` identifier is the branch signal.
const String upcAMessageInvalidScalars =
    'UPC-A input must contain valid Unicode scalar values';
const String upcAMessageNullInput = 'UPC-A input must not be null';

String upcAMessageFor(UpcAErrorCode? code) {
  switch (code) {
    case UpcAErrorCode.inputTooLong:
      return upcAMessageInputTooLong;
    case UpcAErrorCode.invalidLength:
      return upcAMessageInvalidLength;
    case UpcAErrorCode.invalidCharacter:
    case null:
      return upcAMessageInvalidCharacter;
    case UpcAErrorCode.invalidCheckDigit:
      return upcAMessageInvalidCheckDigit;
  }
}
