import 'dart:math';

import 'package:coding_adventures_verilog_lexer/verilog_lexer.dart';
import 'package:test/test.dart';

void main() {
  group('Version Resolution & Grammar Loading', () {
    test('resolves default and valid aliases', () {
      expect(resolveVerilogVersion(null), equals('2005'));
      expect(resolveVerilogVersion(''), equals('2005'));
      expect(resolveVerilogVersion('verilog'), equals('2005'));
      expect(resolveVerilogVersion('2005'), equals('2005'));
      expect(resolveVerilogVersion('verilog2005'), equals('2005'));
      expect(resolveVerilogVersion('1995'), equals('1995'));
      expect(resolveVerilogVersion('verilog1995'), equals('1995'));
      expect(resolveVerilogVersion('2001'), equals('2001'));
      expect(resolveVerilogVersion('verilog2001'), equals('2001'));
      expect(resolveVerilogVersion(' VERILOG2001 '), equals('2001'));
    });

    test('rejects unknown versions with InvalidVerilogVersionException', () {
      expect(
        () => resolveVerilogVersion('2017'),
        throwsA(isA<InvalidVerilogVersionException>()),
      );
      expect(
        () => resolveVerilogVersion('sv'),
        throwsA(isA<InvalidVerilogVersionException>()),
      );
    });

    test('loads precompiled token grammars for all editions', () {
      for (final ver in supportedVerilogVersions) {
        final grammar = loadVerilogTokenGrammar(ver);
        expect(grammar.definitions, isNotEmpty);
      }
    });
  });

  group('Preprocessor Directives & In-Memory Operations', () {
    test('expands simple macros', () {
      const source = '''
`define WIDTH 8
wire [`WIDTH-1:0] data;
''';
      final res = verilogPreprocess(source);
      expect(res, contains('wire [8-1:0] data;'));
      expect(res.split('\n')[0],
          equals('')); // line 1 stripped to preserve line count
      expect(res.split('\n')[1], equals('wire [8-1:0] data;'));
    });

    test('expands parameterized macros with arguments', () {
      const source = '''
`define MAX(a, b) ((a) > (b) ? (a) : (b))
assign y = `MAX(in1, in2);
''';
      final res = verilogPreprocess(source);
      expect(res, contains('assign y = ((in1) > (in2) ? (in1) : (in2));'));
    });

    test('handles nested parentheses in macro arguments', () {
      const source = '''
`define MAX(a, b) ((a) > (b) ? (a) : (b))
assign y = `MAX((p + q), r);
''';
      final res = verilogPreprocess(source);
      expect(res, contains('assign y = (((p + q)) > (r) ? ((p + q)) : (r));'));
    });

    test('handles undef directive', () {
      const source = '''
`define WIDTH 8
`undef WIDTH
wire [`WIDTH-1:0] data;
''';
      final res = verilogPreprocess(source);
      expect(res, contains('wire [`WIDTH-1:0] data;'));
    });

    test('handles ifdef / ifndef / else / endif conditionals', () {
      const source = '''
`define ASIC 1
`ifdef ASIC
wire asic_wire;
`else
wire fpga_wire;
`endif
`ifndef SIMULATION
wire synth_wire;
`else
wire sim_wire;
`endif
''';
      final res = verilogPreprocess(source);
      expect(res, contains('wire asic_wire;'));
      expect(res.contains('wire fpga_wire;'), isFalse);
      expect(res, contains('wire synth_wire;'));
      expect(res.contains('wire sim_wire;'), isFalse);
    });

    test('handles nested ifdef conditionals inside inactive sections', () {
      const source = '''
`ifdef INACTIVE
  `ifdef NESTED
    wire w1;
  `endif
`else
  wire w2;
`endif
''';
      final res = verilogPreprocess(source);
      expect(res.contains('wire w1;'), isFalse);
      expect(res, contains('wire w2;'));
    });

    test(
        'stubs include directives with inline comment and zero filesystem access',
        () {
      const source = '`include "header.vh"';
      final res = verilogPreprocess(source);
      expect(res, equals('/* `include "header.vh" — not resolved */'));
    });

    test('strips timescale directives', () {
      const source = '`timescale 1ns/1ps\nmodule top; endmodule';
      final res = verilogPreprocess(source);
      final lines = res.split('\n');
      expect(lines[0], equals(''));
      expect(lines[1], equals('module top; endmodule'));
    });

    test('accepts predefined macros map', () {
      const source = 'wire [`CLOCK_FREQ-1:0] clk;';
      final res = verilogPreprocess(source, predefined: {'CLOCK_FREQ': '50'});
      expect(res, contains('wire [50-1:0] clk;'));
    });
  });

  group('Preprocessor Security Resilience & Input Validation', () {
    test('enforces maxSourceLength on input', () {
      final big = 'a' * (maxSourceLength + 1);
      expect(
        () => verilogPreprocess(big),
        throwsA(isA<InvalidVerilogSourceException>()),
      );
    });

    test('enforces maxSourceLength on expanded macro bombs (CWE-400)', () {
      final bomb = StringBuffer();
      bomb.writeln('`define HUGE ' + ('X' * 20000));
      bomb.write('wire [');
      for (var i = 0; i < 100; i++) {
        bomb.write('`HUGE + ');
      }
      bomb.writeln('0];');
      expect(
        () => verilogPreprocess(bomb.toString()),
        throwsA(isA<InvalidVerilogSourceException>()),
      );
    });

    test('rejects lone UTF-16 surrogates', () {
      expect(
        () => verilogPreprocess('module \uD800; endmodule'),
        throwsA(isA<InvalidVerilogSourceException>()),
      );
      expect(
        () => verilogPreprocess('module \uDC00; endmodule'),
        throwsA(isA<InvalidVerilogSourceException>()),
      );
    });

    test('rejects conditional nesting depth exceeding ceiling (64)', () {
      final nested = StringBuffer();
      for (var i = 0; i < 70; i++) {
        nested.writeln('`ifdef DEF_$i');
      }
      for (var i = 0; i < 70; i++) {
        nested.writeln('`endif');
      }
      expect(
        () => verilogPreprocess(nested.toString()),
        throwsA(isA<InvalidVerilogSourceException>()),
      );
    });
  });

  group('Tokenization & Lexical Analysis', () {
    test('tokenizes keywords, identifiers, and symbols', () {
      const source = 'module top(input wire clk); assign y = a & b; endmodule';
      final tokens = tokenizeVerilog(source);
      expect(
          tokens.map((t) => t.type),
          containsAll(
              ['KEYWORD', 'NAME', 'LPAREN', 'RPAREN', 'AMP', 'SEMICOLON']));
    });

    test('tokenizes sized and unsized numbers', () {
      const source =
          "wire [7:0] data = 8'hA5; integer count = 42; real r = 3.14;";
      final tokens = tokenizeVerilog(source);
      expect(tokens.any((t) => t.type == 'SIZED_NUMBER' && t.value == "8'hA5"),
          isTrue);
      expect(tokens.any((t) => t.type == 'NUMBER' && t.value == '42'), isTrue);
      expect(tokens.any((t) => t.type == 'REAL_NUMBER' && t.value == '3.14'),
          isTrue);
    });

    test('tokenizes system tasks and escaped identifiers', () {
      const source = r'$display("Hello"); \bus[0] = 1;';
      final tokens = tokenizeVerilog(source);
      expect(tokens.any((t) => t.type == 'SYSTEM_ID' && t.value == r'$display'),
          isTrue);
      expect(
          tokens.any((t) => t.type == 'ESCAPED_IDENT' && t.value == r'\bus[0]'),
          isTrue);
    });

    test('supports raw tokenization and streaming lexer creation', () {
      const source = 'module m; endmodule';
      final rawTokens = tokenizeVerilogRaw(source);
      expect(rawTokens, isNotEmpty);

      final lexer = createVerilogLexer(source);
      expect(lexer.tokenize(), isNotEmpty);

      final rawLexer = createVerilogLexerRaw(source);
      expect(rawLexer.tokenize(), isNotEmpty);
    });

    test('throws VerilogLexicalException on unexpected characters', () {
      expect(
        () => tokenizeVerilog('module m; § endmodule'),
        throwsA(isA<VerilogLexicalException>()),
      );
    });
  });

  group('Standing Rule #20 Diagnostics & Exception Hierarchy', () {
    test('VerilogLexerException hierarchy complies with Rule #20', () {
      const exVer = InvalidVerilogVersionException();
      expect(exVer.code, equals(VerilogLexerErrorCode.invalidVersion));
      expect(exVer.code.id, equals('invalid-version'));
      expect(exVer.message, equals(VerilogLexerMessages.invalidVersion));

      const exSize = InvalidVerilogSourceException.sourceTooLarge();
      expect(exSize.code, equals(VerilogLexerErrorCode.invalidSource));
      expect(exSize.message, equals(VerilogLexerMessages.sourceTooLarge));

      const exSurr = InvalidVerilogSourceException.invalidSurrogates();
      expect(exSurr.code, equals(VerilogLexerErrorCode.invalidSource));
      expect(exSurr.message, equals(VerilogLexerMessages.invalidSurrogates));

      const exDepth = InvalidVerilogSourceException.conditionalDepthExceeded();
      expect(exDepth.code, equals(VerilogLexerErrorCode.invalidSource));
      expect(exDepth.message,
          equals(VerilogLexerMessages.conditionalDepthExceeded));

      const exLex = VerilogLexicalException(line: 3, column: 5);
      expect(exLex.code, equals(VerilogLexerErrorCode.lexicalError));
      expect(exLex.line, equals(3));
      expect(exLex.column, equals(5));
      expect(exLex.toString(), contains('line 3, column 5'));
    });

    test('centralized messages include redacted and invalid placeholders', () {
      expect(VerilogLexerMessages.redacted, equals('<redacted>'));
      expect(VerilogLexerMessages.invalid, equals('<invalid>'));
    });
  });

  group('5-Class Security Resilience Showcase', () {
    test('Class 1: Bounded Memory & Input Envelope Protection', () {
      expect(maxSourceLength, equals(1048576));
      expect(maxConditionalDepth, equals(64));
    });

    test('Class 2: Lexical, Character & Encoding Integrity', () {
      expect(minSurrogateCodePoint, equals(0xD800));
      expect(maxSurrogateCodePoint, equals(0xDFFF));
    });

    test('Class 3: Ambient Authority Isolation', () {
      // In-memory include stub verification
      final res = verilogPreprocess('`include "secret.txt"');
      expect(res, contains('not resolved'));
    });

    test('Class 4: Closed Diagnostic Catalog & Payload Blindness', () {
      const exceptions = <VerilogLexerException>[
        InvalidVerilogVersionException(),
        InvalidVerilogSourceException.sourceTooLarge(),
        InvalidVerilogSourceException.invalidSurrogates(),
        InvalidVerilogSourceException.conditionalDepthExceeded(),
        VerilogLexicalException(),
      ];
      for (final ex in exceptions) {
        expect(ex.message, isNotEmpty);
        expect(ex.message.contains('%s'), isFalse);
        expect(ex.message.contains('<script>'), isFalse);
      }
    });

    test('Class 5: Preprocessor Directive State Integrity', () {
      const source = '''
`ifdef UNDEFINED
  `ifdef NESTED
  `endif
`endif
module m; endmodule
''';
      expect(tokenizeVerilog(source), isNotEmpty);
    });
  });

  group('200-Case Deterministic PRNG Property Test', () {
    test(
        'tokenizes randomized Verilog statement combinations across all editions',
        () {
      final rng = Random(0x564552494C4F47); // 'VERILOG'
      const editions = ['1995', '2001', '2005'];

      for (var i = 0; i < 200; i++) {
        final ver = editions[rng.nextInt(editions.length)];
        final numA = rng.nextInt(100);
        final numB = rng.nextInt(100) + 1;
        final hasMacro = rng.nextBool();

        final source = StringBuffer();
        if (hasMacro) {
          source.writeln('`define VAL_$i $numA');
          source.writeln('module test_$i;');
          source.writeln('wire [7:0] a = `VAL_$i + $numB;');
          source.writeln('endmodule');
        } else {
          source.writeln('module test_$i;');
          source.writeln('wire [7:0] a = $numA + $numB;');
          source.writeln('endmodule');
        }

        final tokens = tokenizeVerilog(source.toString(), version: ver);
        expect(tokens, isNotEmpty);
        expect(tokens.first.type, equals('KEYWORD'));
        expect(tokens.first.value, equals('module'));
        expect(tokens.last.type, equals('EOF'));
      }
    });
  });
}
