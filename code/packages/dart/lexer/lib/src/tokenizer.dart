import 'token.dart';
import 'tokenizer_dfa.dart';

class LexerConfig {
  const LexerConfig({required this.keywords});

  final List<String> keywords;
}

class LexerError implements Exception {
  LexerError(this.message, this.line, this.column);

  final String message;
  final int line;
  final int column;

  @override
  String toString() => 'Lexer error at $line:$column: $message';
}

const Map<String, String> _simpleTokens = <String, String>{
  '+': 'PLUS',
  '-': 'MINUS',
  '*': 'STAR',
  '/': 'SLASH',
  '(': 'LPAREN',
  ')': 'RPAREN',
  ',': 'COMMA',
  ':': 'COLON',
  ';': 'SEMICOLON',
  '{': 'LBRACE',
  '}': 'RBRACE',
  '[': 'LBRACKET',
  ']': 'RBRACKET',
  '.': 'DOT',
  '!': 'BANG',
};

bool _isDigit(String char) => char.compareTo('0') >= 0 && char.compareTo('9') <= 0;

bool _isAlpha(String char) {
  return (char.compareTo('a') >= 0 && char.compareTo('z') <= 0) ||
      (char.compareTo('A') >= 0 && char.compareTo('Z') <= 0) ||
      char == '_';
}

bool _isAlphaNumeric(String char) => _isAlpha(char) || _isDigit(char);

List<Token> tokenize(String source, [LexerConfig? config]) {
  var position = 0;
  var line = 1;
  var column = 1;
  final tokens = <Token>[];
  final keywordSet = Set<String>.from(config?.keywords ?? const <String>[]);

  String? currentChar() => position < source.length ? source[position] : null;

  String advance() {
    final char = source[position];
    position += 1;
    if (char == '\n') {
      line += 1;
      column = 1;
    } else {
      column += 1;
    }
    return char;
  }

  void skipWhitespace() {
    while (true) {
      final current = currentChar();
      if (current == null || !' \t\r'.contains(current)) {
        break;
      }
      advance();
    }
  }

  Token readNumber() {
    final startLine = line;
    final startColumn = column;
    final digits = StringBuffer();
    while (currentChar() != null && _isDigit(currentChar()!)) {
      digits.write(advance());
    }
    return Token(
      type: 'NUMBER',
      value: digits.toString(),
      line: startLine,
      column: startColumn,
    );
  }

  Token readName() {
    final startLine = line;
    final startColumn = column;
    final buffer = StringBuffer();
    while (currentChar() != null && _isAlphaNumeric(currentChar()!)) {
      buffer.write(advance());
    }
    final name = buffer.toString();
    return Token(
      type: keywordSet.contains(name) ? 'KEYWORD' : 'NAME',
      value: name,
      line: startLine,
      column: startColumn,
    );
  }

  Token readString() {
    final startLine = line;
    final startColumn = column;
    final buffer = StringBuffer();
    advance();

    while (true) {
      final current = currentChar();
      if (current == null) {
        throw LexerError('Unterminated string literal', startLine, startColumn);
      }
      if (current == '"') {
        advance();
        break;
      }
      if (current == r'\') {
        advance();
        final escaped = currentChar();
        if (escaped == null) {
          throw LexerError(
            'Unterminated string literal (ends with backslash)',
            startLine,
            startColumn,
          );
        }
        const escapeMap = <String, String>{
          'n': '\n',
          't': '\t',
          r'\': r'\',
          '"': '"',
        };
        buffer.write(escapeMap[escaped] ?? escaped);
        advance();
      } else {
        buffer.write(current);
        advance();
      }
    }

    return Token(
      type: 'STRING',
      value: buffer.toString(),
      line: startLine,
      column: startColumn,
    );
  }

  final dfa = newTokenizerDfa();
  while (true) {
    final char = currentChar();
    final nextState = dfa.process(classifyChar(char));
    if (nextState == 'at_whitespace') {
      skipWhitespace();
    } else if (nextState == 'at_newline') {
      tokens.add(Token(type: 'NEWLINE', value: r'\n', line: line, column: column));
      advance();
    } else if (nextState == 'in_number') {
      tokens.add(readNumber());
    } else if (nextState == 'in_name') {
      tokens.add(readName());
    } else if (nextState == 'in_string') {
      tokens.add(readString());
    } else if (nextState == 'in_equals') {
      final startLine = line;
      final startColumn = column;
      advance();
      if (currentChar() == '=') {
        advance();
        tokens.add(
          Token(
            type: 'EQUALS_EQUALS',
            value: '==',
            line: startLine,
            column: startColumn,
          ),
        );
      } else {
        tokens.add(
          Token(
            type: 'EQUALS',
            value: '=',
            line: startLine,
            column: startColumn,
          ),
        );
      }
    } else if (nextState == 'in_operator') {
      tokens.add(
        Token(
          type: _simpleTokens[char]!,
          value: char!,
          line: line,
          column: column,
        ),
      );
      advance();
    } else if (nextState == 'done') {
      break;
    } else if (nextState == 'error') {
      throw LexerError(
        'Unexpected character: ${_quote(char)}',
        line,
        column,
      );
    }
    dfa.reset();
  }

  tokens.add(Token(type: 'EOF', value: '', line: line, column: column));
  return List<Token>.unmodifiable(tokens);
}

String _quote(String? value) => value == null ? 'null' : '"$value"';
