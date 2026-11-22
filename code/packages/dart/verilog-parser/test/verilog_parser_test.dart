import 'dart:math';

import 'package:coding_adventures_verilog_parser/verilog_parser.dart';
import 'package:test/test.dart';

void main() {
  group('Version Resolution & Grammar Loading', () {
    test('resolves default and valid version aliases', () {
      expect(resolveVerilogParserVersion(), equals('verilog'));
      expect(resolveVerilogParserVersion(''), equals('verilog'));
      expect(resolveVerilogParserVersion('verilog'), equals('verilog'));
      expect(resolveVerilogParserVersion('default'), equals('verilog'));
      expect(resolveVerilogParserVersion('2005'), equals('2005'));
      expect(resolveVerilogParserVersion('verilog2005'), equals('2005'));
      expect(resolveVerilogParserVersion('1995'), equals('1995'));
      expect(resolveVerilogParserVersion('verilog1995'), equals('1995'));
      expect(resolveVerilogParserVersion('2001'), equals('2001'));
      expect(resolveVerilogParserVersion('verilog2001'), equals('2001'));
      expect(resolveVerilogParserVersion('  VERILOG2005  '), equals('2005'));
    });

    test('rejects unknown or invalid version strings', () {
      expect(
        () => resolveVerilogParserVersion('verilog2020'),
        throwsA(isA<InvalidVerilogParserVersionException>()),
      );
      expect(
        () => resolveVerilogParserVersion('systemverilog'),
        throwsA(isA<InvalidVerilogParserVersionException>()),
      );
    });

    test('loads precompiled parser grammars for all editions', () {
      for (final ver in supportedVerilogParserVersions) {
        final grammar = loadVerilogParserGrammar(version: ver);
        expect(grammar.rules, isNotEmpty);
      }
      final defaultGrammar = loadVerilogParserGrammar();
      expect(defaultGrammar.rules, isNotEmpty);
    });
  });

  group('Verilog Syntactic Parsing', () {
    test('parses empty module declaration', () {
      const source = 'module empty; endmodule';
      final ast = parseVerilog(source);
      expect(ast.ruleName, equals('source_text'));
      expect(ast.children, isNotEmpty);
    });

    test('parses module with port declarations and widths', () {
      const source = '''
module counter(input wire clk, output reg [7:0] q);
endmodule
''';
      final ast = parseVerilog(source);
      expect(ast.ruleName, equals('source_text'));
    });

    test('parses continuous assignments', () {
      const source = '''
module adder;
  wire a, b, sum, carry;
  assign sum = a ^ b;
  assign carry = a & b;
endmodule
''';
      final ast = parseVerilog(source);
      expect(ast.ruleName, equals('source_text'));
    });

    test('parses sequential always blocks with sensitivity lists', () {
      const source = '''
module dff;
  reg q;
  wire clk, rst, d;
  always @(posedge clk or posedge rst) begin
    if (rst) begin
      q <= 1'b0;
    end else begin
      q <= d;
    end
  end
endmodule
''';
      final ast = parseVerilog(source);
      expect(ast.ruleName, equals('source_text'));
    });

    test('parses case statements', () {
      const source = '''
module mux4;
  reg y;
  wire [1:0] sel;
  wire a, b, c, d;
  always @(*) begin
    case (sel)
      2'b00: y = a;
      2'b01: y = b;
      2'b10: y = c;
      default: y = d;
    endcase
  end
endmodule
''';
      final ast = parseVerilog(source);
      expect(ast.ruleName, equals('source_text'));
    });

    test('parses module instantiations', () {
      const source = '''
module top;
  wire clk, d, q;
  submod u1 (.clk(clk), .d(d), .q(q));
endmodule
''';
      final ast = parseVerilog(source);
      expect(ast.ruleName, equals('source_text'));
    });

    test('parses generate blocks and genvars', () {
      const source = '''
module gen_demo;
  generate
    genvar i;
  endgenerate
endmodule
''';
      final ast = parseVerilog(source);
      expect(ast.ruleName, equals('source_text'));
    });

    test('parses functions and tasks', () {
      const source = '''
module subprograms;
  function [7:0] invert;
    input [7:0] in;
    invert = ~in;
  endfunction

  task do_reset;
    reg r;
    r = 1'b0;
  endtask
endmodule
''';
      final ast = parseVerilog(source);
      expect(ast.ruleName, equals('source_text'));
    });

    test('integrates transparently with preprocessor macros', () {
      const source = '''
`define WIDTH 8
module parameterized(input wire [`WIDTH-1:0] data_in);
endmodule
''';
      final ast = parseVerilog(source);
      expect(ast.ruleName, equals('source_text'));
    });

    test('supports parser factory and token-stream APIs', () {
      const source = 'module helper; endmodule';
      final p1 = createVerilogParser(source);
      expect(p1.parse().ruleName, equals('source_text'));

      final p2 = createVerilogParserFromTokens(p1.tokens);
      expect(p2.parse().ruleName, equals('source_text'));

      final ast = parseVerilogTokens(p1.tokens);
      expect(ast.ruleName, equals('source_text'));
    });
  });

  group('Standing Rule #20: Centralized Diagnostics & Payload Blindness', () {
    test('reports syntax error without reflecting caller code', () {
      const badSource = 'module 123_invalid;';
      try {
        parseVerilog(badSource);
        fail('Expected syntax error exception');
      } on VerilogParseException catch (e) {
        expect(e.errorCode, equals(VerilogParserErrorCode.syntaxError));
        expect(e.errorId, equals('syntax-error'));
        expect(e.message, equals(messageSyntaxError));
        expect(e.toString(), contains('syntax parse error'));
        // Payload-blindness check: no code fragment leaked
        expect(e.toString().contains('123_invalid'), isFalse);
        expect(e.toString().contains('%s'), isFalse);
        expect(e.toString().contains('%d'), isFalse);
      }
    });

    test('sealed exception closed constructors and error codes', () {
      const ex1 = InvalidVerilogParserVersionException();
      expect(ex1.errorCode, equals(VerilogParserErrorCode.invalidVersion));
      expect(ex1.errorId, equals('invalid-version'));
      expect(ex1.message, equals(messageInvalidVersion));

      const ex2 = VerilogParseException.tokenCountExceeded();
      expect(ex2.errorCode, equals(VerilogParserErrorCode.tokenCountExceeded));
      expect(ex2.errorId, equals('token-count-exceeded'));
      expect(ex2.message, equals(messageTokenCountExceeded));

      const ex3 = VerilogParseException.nestingTooDeep();
      expect(ex3.errorCode, equals(VerilogParserErrorCode.nestingTooDeep));
      expect(ex3.errorId, equals('nesting-too-deep'));
      expect(ex3.message, equals(messageNestingTooDeep));

      const ex4 = VerilogParseException.syntaxError(line: 5, column: 12);
      expect(ex4.errorCode, equals(VerilogParserErrorCode.syntaxError));
      expect(ex4.errorId, equals('syntax-error'));
      expect(ex4.line, equals(5));
      expect(ex4.column, equals(12));
      expect(ex4.toString(), contains('(5:12)'));
    });

    test('formatLocation handles null and non-null offsets', () {
      expect(formatLocation('msg', null, null), equals('msg'));
      expect(formatLocation('msg', 1, null), equals('msg'));
      expect(formatLocation('msg', null, 2), equals('msg'));
      expect(formatLocation('msg', 3, 4), equals('msg (3:4)'));
    });

    test('placeholders match specification', () {
      expect(redactedPlaceholder, equals('<redacted>'));
      expect(invalidPlaceholder, equals('<invalid>'));
    });
  });

  group('5-Class Security Resilience Showcase', () {
    test('Class 1: Bounded Memory via Token Ceiling Protection', () {
      final oversizedTokens = List<Token>.generate(
        maxTokenCount + 1,
        (i) => Token(type: 'SEMICOLON', value: ';', line: 1, column: i + 1),
      );

      expect(
        () => VerilogParser.fromTokens(oversizedTokens),
        throwsA(
          isA<VerilogParseException>().having(
            (e) => e.errorId,
            'errorId',
            equals(errorIdTokenCountExceeded),
          ),
        ),
      );

      final atLimitTokens = List<Token>.generate(
        maxTokenCount,
        (i) => Token(type: 'SEMICOLON', value: ';', line: 1, column: i + 1),
      );
      final parser = VerilogParser.fromTokens(atLimitTokens);
      expect(parser.tokens.length, equals(maxTokenCount));
    });

    test('Class 2a: Delimiter Nesting Depth (Parentheses)', () {
      final deepParens =
          'module m; assign y = ${"(" * 65}1${")" * 65}; endmodule';
      expect(
        () => parseVerilog(deepParens),
        throwsA(
          isA<VerilogParseException>().having(
            (e) => e.errorId,
            'errorId',
            equals(errorIdNestingTooDeep),
          ),
        ),
      );
    });

    test('Class 2b: Delimiter Nesting Depth (Brackets & Braces)', () {
      final deepBrackets =
          'module m; wire ${"[" * 65}0${"]" * 65} sig; endmodule';
      expect(
        () => parseVerilog(deepBrackets),
        throwsA(isA<VerilogParseException>()),
      );

      final deepBraces =
          'module m; assign y = ${"{" * 65}1${"}" * 65}; endmodule';
      expect(
        () => parseVerilog(deepBraces),
        throwsA(isA<VerilogParseException>()),
      );
    });

    test('Class 2c: Scoped Keyword Block Nesting Depth (begin / end)', () {
      final openBlocks = List.filled(65, 'begin').join(' ');
      final closeBlocks = List.filled(65, 'end').join(' ');
      final deepBlocks = '''
module m;
  initial begin
    $openBlocks
      y = 1;
    $closeBlocks
  end
endmodule
''';
      expect(
        () => parseVerilog(deepBlocks),
        throwsA(
          isA<VerilogParseException>().having(
            (e) => e.errorId,
            'errorId',
            equals(errorIdNestingTooDeep),
          ),
        ),
      );
    });

    test('Class 2d: Consecutive Unary Prefix Operator Chains', () {
      final deepOps = 'module m; assign y = ${"!" * 65}1; endmodule';
      expect(
        () => parseVerilog(deepOps),
        throwsA(
          isA<VerilogParseException>().having(
            (e) => e.errorId,
            'errorId',
            equals(errorIdNestingTooDeep),
          ),
        ),
      );
    });

    test('Class 2e: StackOverflowError Backstop Recovery', () {
      final options = GrammarParserOptions(
        trace: true,
        traceWriter: (_) => throw StackOverflowError(),
      );
      expect(
        () => parseVerilog('module m; endmodule', options: options),
        throwsA(
          isA<VerilogParseException>().having(
            (e) => e.errorId,
            'errorId',
            equals(errorIdNestingTooDeep),
          ),
        ),
      );
    });

    test('Class 3: AST Structural Hierarchy & Node Integrity', () {
      const source = 'module m; wire w; endmodule';
      final ast = parseVerilog(source);
      expect(ast.ruleName, equals('source_text'));
      expect(ast.children, isNotEmpty);
      final desc = ast.children.first as ASTNode;
      expect(desc.ruleName, equals('description'));
      final modDecl = desc.children.first as ASTNode;
      expect(modDecl.ruleName, equals('module_declaration'));
    });

    test('Class 4: Closed Diagnostic Catalog & Payload Blindness', () {
      const exceptions = <VerilogParserException>[
        InvalidVerilogParserVersionException(),
        VerilogParseException.tokenCountExceeded(),
        VerilogParseException.nestingTooDeep(),
        VerilogParseException.syntaxError(),
      ];
      for (final ex in exceptions) {
        expect(ex.message, isNotEmpty);
        expect(ex.message.contains('%s'), isFalse);
        expect(ex.message.contains('%d'), isFalse);
        expect(ex.message.contains('<script>'), isFalse);
      }
    });

    test('Class 5: Adversarial Malformed Input Resilience', () {
      const malformedCases = [
        'module',
        'module ;',
        'module m (',
        'module m begin end',
        'endmodule',
        'assign = ;',
        'always @()',
      ];
      for (final bad in malformedCases) {
        expect(
          () => parseVerilog(bad),
          throwsA(isA<VerilogParserException>()),
        );
      }
    });
  });

  group('200-Case Deterministic PRNG Property Test', () {
    test('cross-validates randomized Verilog constructs across all editions',
        () {
      final rng = Random(0x564552494C4F47); // 'VERILOG'
      const portTypes = ['wire', 'reg', 'tri'];
      const directions = ['input', 'output', 'inout'];
      const operators = ['&', '|', '^', '+', '-', '&&', '||'];

      for (var i = 0; i < 200; i++) {
        final edition = supportedVerilogParserVersions[
            rng.nextInt(supportedVerilogParserVersions.length)];
        final hasPorts = rng.nextBool();
        final hasAssign = rng.nextBool();
        final modName = 'mod_${i}_${rng.nextInt(1000)}';

        final buffer = StringBuffer();
        buffer.write('module $modName');

        if (hasPorts && edition != '1995') {
          final dir = directions[rng.nextInt(directions.length)];
          final pt = portTypes[rng.nextInt(portTypes.length)];
          buffer.write('($dir $pt [7:0] port_a)');
        }
        buffer.writeln(';');

        if (hasAssign) {
          final op = operators[rng.nextInt(operators.length)];
          buffer.writeln('  wire w1, w2, w3;');
          buffer.writeln('  assign w3 = w1 $op w2;');
        }

        buffer.writeln('endmodule');

        final source = buffer.toString();
        final ast = parseVerilog(source, version: edition);
        expect(ast.ruleName, equals('source_text'));
      }
    });
  });
}
