class Token {
  const Token({
    required this.type,
    required this.value,
    required this.line,
    required this.column,
    this.flags,
  });

  final String type;
  final String value;
  final int line;
  final int column;
  final int? flags;

  @override
  bool operator ==(Object other) {
    return other is Token &&
        other.type == type &&
        other.value == value &&
        other.line == line &&
        other.column == column &&
        other.flags == flags;
  }

  @override
  int get hashCode => Object.hash(type, value, line, column, flags);

  @override
  String toString() {
    return 'Token(type: $type, value: $value, line: $line, column: $column, flags: $flags)';
  }
}

const int TOKEN_PRECEDED_BY_NEWLINE = 1;
const int TOKEN_CONTEXT_KEYWORD = 2;
