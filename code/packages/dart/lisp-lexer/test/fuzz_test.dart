/// Hostile-input fuzz campaign for `coding_adventures_lisp_lexer`, using
/// the in-lane `fuzz` package: byte strings decode to hostile source
/// text driving [tokenize], and the model asserts the fail-closed
/// vocabulary (only [LexerError] escapes), the EOF invariant, and the
/// space-join fixed point. Fixed seed, so the campaign reproduces
/// exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_lisp_lexer/lisp_lexer.dart';
import 'package:test/test.dart';

/// Decodes hostile bytes to source text: runs of ASCII plus injected
/// astral, surrogate, and structural characters.
String decodeHostile(Uint8List input) {
  final units = <int>[];
  for (var i = 0; i < input.length; i++) {
    final b = input[i];
    // Byte 0x00 escapes into the next byte as a special: NUL itself is a
    // valid error character on its own, so route through it sparingly.
    if (b == 0x00 && i + 1 < input.length) {
      final selector = input[i + 1] % 8;
      i++;
      switch (selector) {
        case 0:
          units.add(0x22); // quote
        case 1:
          units.add(0x5C); // backslash
        case 2:
          units.add(0x3B); // semicolon
        case 3:
          units.add(0xD800); // lone high surrogate
        case 4:
          units.add(0xDC00); // lone low surrogate
        case 5:
          units.addAll([0xD83D, 0xDE00]); // astral pair
        case 6:
          units.add(0xE9); // latin-1
        default:
          units.add(0x0A); // newline
      }
      continue;
    }
    units.add(b);
  }
  return String.fromCharCodes(units);
}

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  cov.recordEdge(input.length % 64);
  final source = decodeHostile(input);
  cov.recordEdge(source.length % 32);
  List<Token> tokens;
  try {
    tokens = tokenize(source);
  } on LexerError {
    cov.recordEdge(100);
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash('unexpected-escape', ['${e.runtimeType}']);
  }
  cov.recordEdge(101);

  // EOF is last, alone, and empty.
  if (tokens.isEmpty ||
      tokens.last.tokenType != TokenType.eof ||
      tokens.last.value != '') {
    return const OutcomeCrash('model-divergence', ['eof']);
  }
  if (tokens.take(tokens.length - 1).any((t) => t.tokenType == TokenType.eof)) {
    return const OutcomeCrash('model-divergence', ['eof-once']);
  }
  cov.recordEdge(102);

  // Space-joined values re-tokenize identically.
  final joined = tokens.map((t) => t.value).join(' ');
  List<Token> second;
  try {
    second = tokenize(joined);
  } on LexerError {
    return const OutcomeCrash('model-divergence', ['reparse-throws']);
  } catch (e) {
    return OutcomeCrash('unexpected-escape', ['reparse:${e.runtimeType}']);
  }
  if (second.length != tokens.length) {
    return const OutcomeCrash('model-divergence', ['reparse-length']);
  }
  for (var i = 0; i < tokens.length; i++) {
    if (second[i] != tokens[i]) {
      return const OutcomeCrash('model-divergence', ['reparse-value']);
    }
  }
  cov.recordEdge(103 + (tokens.length % 8));
  return const OutcomePass();
}

void main() {
  test('hostile campaign finds no escapes or divergences', () {
    Uint8List b(String s) => Uint8List.fromList(utf8.encode(s));
    final seeds = <Uint8List>[
      Uint8List(0),
      b('(+ 1 2)'),
      b('(define factorial (lambda (n) (cond ((eq n 0) 1))))'),
      b('"hello \\"world\\""'),
      b('; comment only'),
      b(r'@#$'),
      b('""""'),
      b('"unterminated'),
      b('((('),
      b('-42 - a-1 1a'),
      b("'quote . dot"),
      Uint8List.fromList([0, 3, 0, 4, 0x22, 0x5C]),
      Uint8List.fromList(List.filled(64, 0x28)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(1407)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
