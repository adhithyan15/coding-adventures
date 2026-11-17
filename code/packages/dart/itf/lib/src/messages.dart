import 'errors.dart';

/// Diagnostic and error messages for ITF encoding in Dart, strictly payload-blind.
const String itfErrorInputTooLong = 'input-too-long';
const String itfErrorInvalidLength = 'invalid-length';
const String itfErrorInvalidCharacter = 'invalid-character';

const String itfMessageInputTooLong =
    'ITF input must contain at most 4096 characters';
const String itfMessageInvalidLength =
    'ITF input must contain an even number of digits';
const String itfMessageInvalidCharacter = 'ITF input must contain digits only';
const String itfMessageInvalidScalars =
    'ITF input must contain valid Unicode scalar values';
const String itfMessageNullInput = 'ITF input must not be null';

const String itfRedactedPlaceholder = '<redacted>';
const String itfInvalidPlaceholder = '<invalid>';

String itfMessageFor(ItfErrorCode? code) {
  switch (code) {
    case ItfErrorCode.inputTooLong:
      return itfMessageInputTooLong;
    case ItfErrorCode.invalidLength:
      return itfMessageInvalidLength;
    case ItfErrorCode.invalidCharacter:
    case null:
      return itfMessageInvalidCharacter;
  }
}
