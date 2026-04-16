class CliBuilderError implements Exception {
  CliBuilderError(this.message);

  final String message;

  @override
  String toString() => message;
}

class SpecError extends CliBuilderError {
  SpecError(super.message);
}

class ParseError extends CliBuilderError {
  ParseError({
    required this.errorType,
    required String message,
    this.suggestion,
    this.context = const <String>[],
  }) : super(message);

  final String errorType;
  final String? suggestion;
  final List<String> context;
}

class ParseErrors extends CliBuilderError {
  ParseErrors(this.errors)
      : super(
          errors.length == 1
              ? 'parse error: ${errors.first.message}'
              : '${errors.length} parse errors:\n'
                  '${errors.map((error) => '  - ${error.message}').join('\n')}',
        );

  final List<ParseError> errors;
}

class ValidationResult {
  const ValidationResult({
    required this.errors,
    this.warnings = const <String>[],
  });

  final List<String> errors;
  final List<String> warnings;

  bool get isValid => errors.isEmpty;
}
