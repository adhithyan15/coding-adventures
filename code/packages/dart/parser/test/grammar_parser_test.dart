import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

void main() {
  Token tok(String type, String value, [int line = 1, int column = 1]) {
    return Token(type: type, value: value, line: line, column: column);
  }

  Token eof([int line = 1, int column = 1]) {
    return Token(type: 'EOF', value: '', line: line, column: column);
  }

  ParserGrammar grammar(String source) => parseParserGrammar(source);

  group('GrammarParser', () {
    test('parses JSON-like structures into AST nodes', () {
      final parser = GrammarParser(
        [
          tok('LBRACE', '{'),
          tok('STRING', 'name', 1, 2),
          tok('COLON', ':', 1, 8),
          tok('STRING', 'Alice', 1, 10),
          tok('RBRACE', '}', 1, 17),
          eof(1, 18),
        ],
        grammar('''
          value = object | STRING ;
          object = LBRACE pair RBRACE ;
          pair = STRING COLON value ;
        '''),
      );

      final ast = parser.parse();

      expect(ast.ruleName, 'value');
      expect(findNodes(ast, 'object'), hasLength(1));
      expect(findNodes(ast, 'pair'), hasLength(1));
      expect(collectTokens(ast, type: 'STRING').map((token) => token.value), [
        'name',
        'Alice',
      ]);
      expect(ast.startLine, 1);
      expect(ast.startColumn, 1);
      expect(ast.endLine, 1);
      expect(ast.endColumn, 17);
    });

    test('skips insignificant newlines', () {
      final parser = GrammarParser([
        tok('NEWLINE', '\n'),
        tok('NUMBER', '42', 2, 1),
        eof(2, 3),
      ], grammar('value = NUMBER ;'));

      final ast = parser.parse();
      expect(ast.ruleName, 'value');
      expect(getLeafToken(ast), isNotNull);
      expect(getLeafToken(ast)!.value, '42');
    });

    test('treats newlines as significant when grammar references them', () {
      final parser = GrammarParser([
        tok('NUMBER', '1'),
        tok('NEWLINE', '\n', 1, 2),
        tok('NUMBER', '2', 2, 1),
        eof(2, 2),
      ], grammar('program = NUMBER NEWLINE NUMBER ;'));

      final ast = parser.parse();
      expect(ast.ruleName, 'program');
      expect(collectTokens(ast).map((token) => token.type), [
        'NUMBER',
        'NEWLINE',
        'NUMBER',
      ]);
    });

    test('matches literal tokens by value', () {
      final parser = GrammarParser([
        tok('KEYWORD', 'let'),
        tok('NAME', 'answer', 1, 5),
        eof(1, 11),
      ], grammar('binding = "let" NAME ;'));

      final ast = parser.parse();
      expect(ast.ruleName, 'binding');
      expect(collectTokens(ast).map((token) => token.value), ['let', 'answer']);
    });

    test('walkAST can rewrite nodes', () {
      final parser = GrammarParser([
        tok('NUMBER', '42', 3, 4),
        eof(3, 6),
      ], grammar('value = NUMBER ;'));

      final ast = parser.parse();
      final rewritten = walkAST(
        ast,
        ASTVisitor(
          leave: (node, _) {
            if (node.ruleName == 'value') {
              return ASTNode(
                ruleName: 'literal',
                children: node.children,
                startLine: node.startLine,
                startColumn: node.startColumn,
                endLine: node.endLine,
                endColumn: node.endColumn,
              );
            }
            return null;
          },
        ),
      );

      expect(rewritten.ruleName, 'literal');
      expect(collectTokens(rewritten).single.value, '42');
    });

    test('reports parse errors with the furthest token', () {
      final parser = GrammarParser([
        tok('LBRACKET', '[', 4, 1),
        tok('NUMBER', '1', 4, 2),
        tok('COMMA', ',', 4, 3),
        tok('RBRACKET', ']', 4, 4),
        eof(4, 5),
      ], grammar('array = LBRACKET NUMBER COMMA NUMBER RBRACKET ;'));

      expect(
        () => parser.parse(),
        throwsA(
          isA<GrammarParseError>().having(
            (error) => error.token?.column,
            'token column',
            4,
          ),
        ),
      );
    });
  });
}
