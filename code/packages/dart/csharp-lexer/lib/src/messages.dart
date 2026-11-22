import 'errors.dart';

/// Diagnostic and error messages for C# lexical analysis, strictly payload-blind.
const String csharpLexerErrorInvalidVersion = 'invalid-version';
const String csharpLexerErrorInvalidSource = 'invalid-source';
const String csharpLexerErrorLexicalError = 'lexical-error';

const String messageInvalidVersion = 'Unknown or unsupported C# version';
const String messageSourceTooLarge = 'C# source exceeds maximum allowed size';
const String messageInvalidSurrogates =
    'C# source contains invalid Unicode surrogates';
const String messageLexicalError =
    'Lexical analysis failed due to unrecognized token';

const String redactedPlaceholder = '<redacted>';
const String invalidPlaceholder = '<invalid>';

/// Returns the static error message associated with an error code.
///
/// `invalidSource` maps to the size-ceiling message, matching the Swift lane.
/// The surrogate variant is Dart-only (Swift `String` cannot hold lone
/// surrogates) and is raised by `InvalidCSharpSourceException.invalidSurrogates`.
String messageFor(CSharpLexerErrorCode code) {
  switch (code) {
    case CSharpLexerErrorCode.invalidVersion:
      return messageInvalidVersion;
    case CSharpLexerErrorCode.invalidSource:
      return messageSourceTooLarge;
    case CSharpLexerErrorCode.lexicalError:
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
