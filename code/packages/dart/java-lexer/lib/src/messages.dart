import 'errors.dart';

/// Diagnostic and error messages for Java lexical analysis, strictly payload-blind.
const String javaLexerErrorInvalidVersion = 'invalid-version';
const String javaLexerErrorInvalidSource = 'invalid-source';
const String javaLexerErrorLexicalError = 'lexical-error';

const String messageInvalidVersion = 'Unknown or unsupported Java version';
const String messageSourceTooLarge = 'Java source exceeds maximum allowed size';
const String messageInvalidSurrogates =
    'Java source contains invalid Unicode surrogates';
const String messageLexicalError =
    'Lexical analysis failed due to unrecognized token';

const String redactedPlaceholder = '<redacted>';
const String invalidPlaceholder = '<invalid>';

/// Returns the static error message associated with an error code.
///
/// `invalidSource` maps to the size-ceiling message, matching the Swift lane.
/// The surrogate variant is Dart-only (Swift `String` cannot hold lone
/// surrogates) and is raised by `InvalidJavaSourceException.invalidSurrogates`.
String messageFor(JavaLexerErrorCode code) {
  switch (code) {
    case JavaLexerErrorCode.invalidVersion:
      return messageInvalidVersion;
    case JavaLexerErrorCode.invalidSource:
      return messageSourceTooLarge;
    case JavaLexerErrorCode.lexicalError:
      return messageLexicalError;
  }
}

/// Formats an error message with 1-based line and column offsets if available.
String formatLocation(String message, int? line, int? column) {
  if (line != null && column != null) {
    return '$message ($line:$column)';
  }
  return message;
}
