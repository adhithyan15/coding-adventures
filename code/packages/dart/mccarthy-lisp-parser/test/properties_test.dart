/// Property tests for `coding_adventures_mccarthy_lisp_parser`,
/// using the in-lane `pbt` package: the failure vocabulary over
/// hostile text, determinism, the display fixpoint, atom and
/// valid-program parsing, both exact depth boundaries across wide
/// ranges (list 59/60 binding the engine, quote 64/65 binding the
/// prescan), API agreement, error positions, and constructor laws —
/// all with fixed seeds.
library;

import 'package:coding_adventures_mccarthy_lisp_parser/mccarthy_lisp_parser.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

/// Structural form-sequence equality (`List.==` is identity).
bool formsEqual(List<LispExpr> a, List<LispExpr> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

/// Maximum open-paren depth of [s].
int maxParenDepthOf(String s) {
  var depth = 0;
  var max = 0;
  for (var i = 0; i < s.length; i++) {
    final c = s.codeUnitAt(i);
    if (c == 0x28) {
      depth++;
      if (depth > max) max = depth;
    } else if (c == 0x29) {
      depth--;
    }
  }
  return max;
}

/// McCarthy-dense hostile text: every Lisp sigil plus lowercase,
/// digits, operators, strings, whitespace, comments, and
/// out-of-alphabet noise.
Gen<String> hostileMcCarthy(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    "'",
    '.',
    ';',
    ' ',
    '\n',
    '\t',
    '\r',
    'A',
    'Z',
    'NIL',
    'CAR',
    'QUOTE',
    'a',
    'z',
    '0',
    '9',
    '-',
    '+',
    '"',
    '[',
    ']',
    '_',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

/// Uppercase symbol bodies; some draws are not valid symbols (leading
/// digit or dash) and are filtered by the caller.
Gen<String> symbolBody() {
  const pieces = ['A', 'B', 'X', 'CAR', 'FOO', 'NIL', '-', '0', '9'];
  return Gen.list(1, 4, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

final _symbolPattern = RegExp(r'^[A-Z][A-Z0-9-]*$');

/// Valid atoms: well-formed symbols plus small and extreme integers.
Gen<String> validAtom() => Gen.oneOf([
      symbolBody().map((s) => _symbolPattern.hasMatch(s) ? s : 'ATOM'),
      smallInt(-999999, 999999).map((i) => '$i'),
      Gen.constant('9223372036854775807'),
      Gen.constant('-9223372036854775808'),
      Gen.constant('0'),
    ]);

/// Small valid S-expressions: atoms, singletons, pairs, dotted pairs,
/// and quotes.
Gen<String> validSexpr() => Gen.oneOf([
      validAtom(),
      validAtom().map((a) => '($a)'),
      Gen.tuple2(validAtom(), validAtom()).map((t) => '(${t.$1} ${t.$2})'),
      Gen.tuple2(validAtom(), validAtom()).map((t) => '(${t.$1} . ${t.$2})'),
      validAtom().map((a) => "'$a"),
    ]);

/// Small expressions: atoms, nil, and one-level lists of atoms.
Gen<LispExpr> smallExpr() => Gen.oneOf([
      Gen.constant(const LispNil()),
      smallInt(-99, 99).map((i) => LispInt(i)),
      symbolBody()
          .map((s) => LispExpr.sym(_symbolPattern.hasMatch(s) ? s : 'ATOM')),
      Gen.list(0, 3, smallInt(-99, 99))
          .map((items) => LispExpr.list(items.map((i) => LispInt(i)))),
    ]);

void main() {
  group('properties', () {
    test('parse fails closed with the wrapped error only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(5801),
          PbtOptions(numTests: 400, seed: BigInt.from(5801)),
          (rng) => hostileMcCarthy(0, 60).sample(rng),
          (input) {
            try {
              parseMcCarthy(input);
            } on McCarthyParseError {
              return true;
            }
            return true;
          },
        ),
      );
    });

    test('nullable twin never throws', () {
      check(
        'twin-total',
        checkProperty<String>(
          BigInt.from(5802),
          PbtOptions(numTests: 400, seed: BigInt.from(5802)),
          (rng) => hostileMcCarthy(0, 60).sample(rng),
          (input) {
            tryParseMcCarthy(input);
            return true;
          },
        ),
      );
    });

    test('parsing is deterministic', () {
      check(
        'determinism',
        checkProperty<String>(
          BigInt.from(5803),
          PbtOptions(numTests: 400, seed: BigInt.from(5803)),
          (rng) => hostileMcCarthy(0, 60).sample(rng),
          (input) {
            Object? first;
            Object? second;
            try {
              first = parseMcCarthy(input);
            } on McCarthyParseError catch (e) {
              first = e;
            }
            try {
              second = parseMcCarthy(input);
            } on McCarthyParseError catch (e) {
              second = e;
            }
            if (first is List<LispExpr> && second is List<LispExpr>) {
              return formsEqual(first, second);
            }
            return first == second;
          },
        ),
      );
    });

    test('display re-parses exactly under the engine list cap', () {
      // `()` displays as `NIL`, which re-parses as the *symbol* NIL,
      // so display is not an equality round-trip — but it is stable
      // when it re-parses at all. Displayed output is always
      // grammatically valid (only parens, dots, atoms) and quoteless,
      // so only the depth caps can reject it, and the engine list cap
      // (59) binds first: re-parsing succeeds iff the display nests
      // at most 59 deep, and then displays identically. (A flat
      // source list displays one paren level per element, so deep
      // displays from shallow sources are expected, not a bug.)
      check(
        'display-fixpoint',
        checkProperty<String>(
          BigInt.from(5804),
          PbtOptions(numTests: 400, seed: BigInt.from(5804)),
          (rng) => hostileMcCarthy(0, 120).sample(rng),
          (input) {
            late final List<LispExpr> forms;
            try {
              forms = parseMcCarthy(input);
            } on McCarthyParseError {
              return true;
            }
            final once = forms.map((f) => f.toString()).join(' ');
            final deep = maxParenDepthOf(once) > 59;
            try {
              final again = parseMcCarthy(once);
              if (deep) return false;
              return again.map((f) => f.toString()).join(' ') == once;
            } on McCarthyParseError {
              return deep;
            }
          },
        ),
      );
    });

    test('valid atoms parse to single forms', () {
      check(
        'atoms-parse',
        checkProperty<String>(
          BigInt.from(5805),
          PbtOptions(numTests: 400, seed: BigInt.from(5805)),
          (rng) => validAtom().sample(rng),
          (input) {
            late final List<LispExpr> forms;
            try {
              forms = parseMcCarthy(input);
            } on McCarthyParseError {
              return false;
            }
            if (forms.length != 1) return false;
            final form = forms.single;
            final asInt = int.tryParse(input);
            if (asInt != null) return form == LispInt(asInt);
            return form == LispExpr.sym(input);
          },
        ),
      );
    });

    test('valid sexprs parse to single forms', () {
      check(
        'sexprs-parse',
        checkProperty<String>(
          BigInt.from(5806),
          PbtOptions(numTests: 400, seed: BigInt.from(5806)),
          (rng) => validSexpr().sample(rng),
          (input) {
            try {
              return parseMcCarthy(input).length == 1;
            } on McCarthyParseError {
              return false;
            }
          },
        ),
      );
    });

    test('list-nesting boundary is exactly 59 over a wide range', () {
      // Full pipeline: the engine cap (59) binds before the prescan
      // (64) for lists, so 60..64 fail with the engine message while
      // 65+ fail in the prescan.
      check(
        'list-boundary',
        checkProperty<int>(
          BigInt.from(5807),
          PbtOptions(numTests: 200, seed: BigInt.from(5807)),
          (rng) => smallInt(0, 120).sample(rng),
          (n) {
            final src = '${'(' * n}A${')' * n}';
            try {
              final forms = parseMcCarthy(src);
              return n <= 59 && forms.length == 1;
            } on McCarthyParseError catch (e) {
              if (n <= 59) return false;
              if (n <= 64) return !e.message.contains('MAX_PAREN_DEPTH');
              return e.message.contains('MAX_PAREN_DEPTH');
            }
          },
        ),
      );
    });

    test('quote-chain boundary is exactly 64 over a wide range', () {
      // Full pipeline: the prescan (64) binds before the engine
      // quote cap (88) for quote chains.
      check(
        'quote-boundary',
        checkProperty<int>(
          BigInt.from(5808),
          PbtOptions(numTests: 200, seed: BigInt.from(5808)),
          (rng) => smallInt(0, 120).sample(rng),
          (n) {
            final src = "${'\'' * n}A";
            try {
              final forms = parseMcCarthy(src);
              return n <= 64 && forms.length == 1;
            } on McCarthyParseError catch (e) {
              if (n <= 64) return false;
              return e.message.contains('MAX_PAREN_DEPTH');
            }
          },
        ),
      );
    });

    test('apis agree and parse success implies cst success', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(5809),
          PbtOptions(numTests: 400, seed: BigInt.from(5809)),
          (rng) => hostileMcCarthy(0, 60).sample(rng),
          (input) {
            final soft = tryParseMcCarthy(input);
            try {
              final hard = parseMcCarthy(input);
              if (soft == null || !formsEqual(soft, hard)) return false;
              // The CST stage succeeded too (the reverse need not
              // hold: integer overflow fails extraction, not the CST).
              return parseMcCarthyToCst(input).ruleName == 'program';
            } on McCarthyParseError {
              return soft == null;
            }
          },
        ),
      );
    });

    test('errors carry positions and messages', () {
      check(
        'error-positions',
        checkProperty<String>(
          BigInt.from(5810),
          PbtOptions(numTests: 400, seed: BigInt.from(5810)),
          (rng) => hostileMcCarthy(0, 60).sample(rng),
          (input) {
            try {
              parseMcCarthy(input);
            } on McCarthyParseError catch (e) {
              return e.message.isNotEmpty && e.line >= 1 && e.column >= 1;
            }
            return true;
          },
        ),
      );
    });

    test('constructor laws hold over small exprs', () {
      check(
        'constructor-laws',
        checkProperty<LispExpr>(
          BigInt.from(5811),
          PbtOptions(numTests: 400, seed: BigInt.from(5811)),
          (rng) => smallExpr().sample(rng),
          (expr) =>
              LispExpr.quote(expr) ==
                  LispExpr.list([LispExpr.sym('QUOTE'), expr]) &&
              LispExpr.list([]) == const LispNil() &&
              expr.toString().isNotEmpty,
        ),
      );
    });
  });
}
