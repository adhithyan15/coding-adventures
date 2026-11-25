/// McCarthy Lisp 1.0 parsing over the compiled grammar: the typed
/// S-expression AST, the throwing and nullable one-shots, the CST
/// entry points, and the CST-to-AST extractor.
///
/// This library is a *thin wrapper*, not a hand-written parser — the
/// Dart lane of the Rust `mccarthy-lisp-parser` crate, powered by the
/// same language definition (`code/grammars/mccarthy_lisp`). Parsing
/// happens in two stages:
///
/// 1. **Grammar parse** — the shared [GrammarParser] turns the token
///    stream into a generic concrete syntax tree ([ASTNode]). All the
///    structural rules — balanced parens, at-most-one dotted tail,
///    "dot must follow an element" — are enforced *here, by the
///    grammar*, so there is no hand-written validation to drift out
///    of sync.
/// 2. **AST extraction** — [extractProgram] lowers that CST to the
///    typed [LispExpr] tree, applying the two sugar expansions:
///    `'X` → `(QUOTE X)` and `(A B C)` → `(A . (B . (C . NIL)))`.
///
/// Like the reference, the *symbol* `NIL` and the empty list `()` are
/// kept distinct at this layer: `()` extracts to [LispNil], while a
/// literal `NIL` token extracts to a [LispSymbol]. Unifying them (as
/// real Lisp does) is a semantic decision left to a later compiler
/// layer.
library;

import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_mccarthy_lisp_lexer/mccarthy_lisp_lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

// ===========================================================================
// AST
// ===========================================================================

/// McCarthy 1960 Lisp S-expression.
///
/// Four cases — matches the abstract grammar of the 1960 paper exactly.
/// Modern Lisps add strings, vectors, hash tables, etc.; McCarthy 1.0
/// has only these four.
sealed class LispExpr {
  const LispExpr();

  /// Convenience: build a [LispSymbol].
  static LispExpr sym(String name) => LispSymbol(name);

  /// Build a McCarthy-style list (`a b c`) as nested [LispCons] cells
  /// terminated by [LispNil]: `(a . (b . (c . NIL)))`.
  static LispExpr list(Iterable<LispExpr> items) {
    LispExpr acc = const LispNil();
    for (final x in items.toList().reversed) {
      acc = LispCons(x, acc);
    }
    return acc;
  }

  /// Wrap an expression in `(QUOTE …)` — the standard expansion of the
  /// `'` reader macro.
  static LispExpr quote(LispExpr inner) =>
      LispExpr.list([LispExpr.sym('QUOTE'), inner]);
}

/// `()` — the empty list, also the false value (any other value is
/// true under McCarthy's `COND`).
final class LispNil extends LispExpr {
  const LispNil();

  @override
  bool operator ==(Object other) => other is LispNil;

  @override
  int get hashCode => 0x4E494C;

  @override
  String toString() => 'NIL';
}

/// `FOO` — an all-uppercase identifier.
final class LispSymbol extends LispExpr {
  const LispSymbol(this.name);

  final String name;

  @override
  bool operator ==(Object other) => other is LispSymbol && other.name == name;

  @override
  int get hashCode => Object.hash(0x53594D, name);

  @override
  String toString() => name;
}

/// `42` — a signed 64-bit integer.
final class LispInt extends LispExpr {
  const LispInt(this.value);

  final int value;

  @override
  bool operator ==(Object other) => other is LispInt && other.value == value;

  @override
  int get hashCode => Object.hash(0x494E54, value);

  @override
  String toString() => '$value';
}

/// `(car . cdr)` — a cons cell, the only compound shape.
final class LispCons extends LispExpr {
  const LispCons(this.car, this.cdr);

  final LispExpr car;
  final LispExpr cdr;

  @override
  bool operator ==(Object other) =>
      other is LispCons && other.car == car && other.cdr == cdr;

  @override
  int get hashCode => Object.hash(0x434F4E, car, cdr);

  @override
  String toString() => '($car . $cdr)';
}

// ===========================================================================
// Errors
// ===========================================================================

/// A McCarthy parse-time error.
///
/// Wraps any of:
/// - a lexer failure (invalid character — e.g. lowercase, a bare `-`),
/// - a [GrammarParser] failure (unbalanced paren, stray dot, …),
/// - an extractor-detected shape problem (integer overflow, depth
///   limit).
///
/// Source positions are 1-indexed and point at the offending token.
/// (Named for the language because the parser engine already owns a
/// `ParseError` type.)
final class McCarthyParseError implements Exception {
  const McCarthyParseError({
    required this.message,
    required this.line,
    required this.column,
  });

  /// Human-readable explanation.
  final String message;

  /// 1-based line number of the offending token.
  final int line;

  /// 1-based column number of the offending token.
  final int column;

  @override
  bool operator ==(Object other) =>
      other is McCarthyParseError &&
      other.message == message &&
      other.line == line &&
      other.column == column;

  @override
  int get hashCode => Object.hash(message, line, column);

  @override
  String toString() => 'parse error at $line:$column: $message';
}

// ===========================================================================
// DoS-hardening depth guards
// ===========================================================================

/// Maximum *structural nesting* depth permitted before the grammar
/// parser runs.
///
/// The downstream [GrammarParser] recursion has its own cap (see
/// [_maxRuleDepth]), but this pre-scan rejects over-deep sources up
/// front by scanning the token stream, exactly like the reference's
/// `MAX_PAREN_DEPTH`.
///
/// **Two token shapes add nesting depth, not just parens:**
///
/// - `LPAREN` — `((((…))))`, via the `list` rule.
/// - `QUOTE`  — `''''…X`, via the `quoted = QUOTE sexpr` rule, which
///   recurses *without consuming a paren*. A long run of `'` has
///   paren-depth 0 yet unbounded parser-recursion depth.
///   [_checkNestingDepth] therefore bounds the *combined* paren +
///   pending-quote depth.
///
/// The cap is **64**, matching the reference. It admits any realistic
/// McCarthy program (the deepest example in the 1960 paper is ~10
/// levels deep).
const int maxParenDepth = 64;

/// Maximum AST-extraction recursion depth.
///
/// A second guard, applied while lowering the CST to [LispExpr], in
/// case a future grammar change lets a deeply-nested tree through the
/// paren check.
const int maxAstDepth = 256;

/// Recursion-depth cap for the [GrammarParser] itself — a *second*,
/// independent layer of defense alongside this library's existing
/// pre-scan ([_checkNestingDepth]/[maxParenDepth]). That pre-scan
/// already rejects excessive combined paren+quote nesting before the
/// parser ever runs, but [createMcCarthyParserFromTokens] is a public
/// entry point (documented for editor/LSP integrations that already
/// hold a token stream) that bypasses it entirely, so a caller
/// invoking it directly with adversarial tokens still needs the
/// engine-level guard.
///
/// Mirrors the reference's measured `MAX_RULE_DEPTH` of 180: list
/// nesting parses cleanly up to 59 levels (60 trips the cap) and
/// quote chains up to 88 levels (89 trips it) — the cap counts rule
/// frames identically in both engines (same grammar cascade, same
/// counting), so both measured boundaries hold here too.
const int _maxRuleDepth = 180;

/// Open-nesting markers for [_checkNestingDepth]'s explicit stack:
/// an open paren (list) or a pending quote. Plain integer constants
/// rather than an enum, whose synthetic constructor would never
/// execute and could never be covered by tests.
const int _parenCtx = 0;
const int _quoteCtx = 1;

/// Reject a source whose maximum *structural nesting* (parens **and**
/// pending quotes) exceeds [maxParenDepth], pointing at the token
/// that tipped it over.
///
/// This runs **before** [GrammarParser.parse], so it is the first
/// thing standing between untrusted input and deep parser recursion.
/// It accounts for *every* token shape that adds parser-recursion
/// depth — both `LPAREN` (the `list` rule) and `QUOTE` (the `quoted`
/// rule, which recurses without a paren).
///
/// An explicit stack of open contexts is tracked. A `QUOTE` opens a
/// pending-quote context that is *discharged* as soon as its single
/// operand sexpr completes (an atom, or a list closed by `RPAREN`); an
/// `LPAREN` opens a list context closed by its `RPAREN`. The
/// high-water mark of the stack is the maximum live recursion depth
/// the grammar parser will reach. (Slight miscounting on *malformed*
/// input is harmless — such input is rejected by the grammar anyway;
/// what matters is never *under*-counting a well-formed deep nest.)
void _checkNestingDepth(List<Token> tokens) {
  final stack = <int>[];

  // Discharge every pending quote sitting on top of the stack: a
  // completed sexpr satisfies all quotes directly wrapping it.
  void dischargeQuotes() {
    while (stack.isNotEmpty && stack.last == _quoteCtx) {
      stack.removeLast();
    }
  }

  for (final t in tokens) {
    switch (t.type) {
      case 'LPAREN':
        stack.add(_parenCtx);
      case 'RPAREN':
        // The list just closed is itself a completed sexpr: pop its
        // paren, then discharge any quotes wrapping the list. Pop
        // down to and including the nearest paren (tolerating stray
        // pending quotes on malformed input).
        while (stack.isNotEmpty && stack.last == _quoteCtx) {
          stack.removeLast();
        }
        if (stack.isNotEmpty && stack.last == _parenCtx) {
          stack.removeLast();
        }
        dischargeQuotes();
      case 'QUOTE':
        stack.add(_quoteCtx);
      case 'SYMBOL' || 'INTEGER':
        // An atom completes a sexpr.
        dischargeQuotes();
      default:
        break;
    }

    if (stack.length > maxParenDepth) {
      throw McCarthyParseError(
        message: 'structural nesting (parens + quotes) exceeds MAX_PAREN_DEPTH '
            '($maxParenDepth) — refusing to invoke the parser to avoid '
            'stack overflow',
        line: t.line,
        column: t.column,
      );
    }
  }
}

void _checkDepth(int depth, int line, int column) {
  if (depth > maxAstDepth) {
    throw McCarthyParseError(
      message: 'AST nesting exceeds MAX_AST_DEPTH ($maxAstDepth) — refusing '
          'to recurse further to avoid stack overflow',
      line: line,
      column: column,
    );
  }
}

// ===========================================================================
// Build-time-compiled parser grammar
// ===========================================================================

/// The build-time-compiled McCarthy Lisp [ParserGrammar].
///
/// Re-exported for tooling that needs to inspect the rules without
/// re-parsing the canonical `.grammar` file. (The reference caches
/// the compiled grammar in a `OnceLock`; a Dart top-level `final` is
/// already lazily initialized exactly once.)
ParserGrammar mccarthyGrammar() => parserGrammar;

// ===========================================================================
// Public API
// ===========================================================================

/// Build a [GrammarParser] from a pre-tokenized stream.
///
/// Useful for editor / LSP integrations that already hold a
/// `List<Token>` (e.g. from `createMcCarthyLexer`). The
/// recursion-depth guard is enabled as a second, independent layer of
/// defense alongside [_checkNestingDepth].
GrammarParser createMcCarthyParserFromTokens(List<Token> tokens) =>
    GrammarParser(
      tokens,
      parserGrammar,
      options: const GrammarParserOptions(maxDepth: _maxRuleDepth),
    );

/// Parse McCarthy Lisp source into the generic [ASTNode] CST.
///
/// The lower-level entry point — most callers want [parseMcCarthy],
/// which goes one step further and returns a typed `List<LispExpr>`.
///
/// Throws [McCarthyParseError] for any lex failure, nesting-limit
/// breach, or grammar mismatch (unbalanced paren, stray dot, …).
ASTNode parseMcCarthyToCst(String source) {
  late final List<Token> tokens;
  try {
    tokens = tokenizeMcCarthy(source);
  } on LexerError catch (e) {
    throw McCarthyParseError(
      message: 'lexer error: ${e.message}',
      line: e.line,
      column: e.column,
    );
  }
  _checkNestingDepth(tokens);
  try {
    return createMcCarthyParserFromTokens(tokens).parse();
  } on GrammarParseError catch (e) {
    final token = e.token;
    throw McCarthyParseError(
      message: e.message,
      line: token?.line ?? 1,
      column: token?.column ?? 1,
    );
  }
}

/// Parse McCarthy 1960 Lisp source into the top-level form sequence.
///
/// A program is zero-or-more S-expressions (McCarthy's "program =
/// sequence of forms" reading).
///
/// Throws [McCarthyParseError] for lex/grammar mismatches *or*
/// extractor-detected shape problems (integer overflow, depth limit).
List<LispExpr> parseMcCarthy(String source) =>
    extractProgram(parseMcCarthyToCst(source));

/// Parse McCarthy 1960 Lisp source, returning `null` instead of
/// throwing on any failure (catch [McCarthyParseError] for the
/// reason). Mirrors the reference's `Result`-returning `parse` with
/// idiomatic Dart nullability.
List<LispExpr>? tryParseMcCarthy(String source) {
  try {
    return parseMcCarthy(source);
  } on McCarthyParseError {
    return null;
  }
}

// ===========================================================================
// CST → typed AST extraction
// ===========================================================================

/// Best-effort source position of a CST node (falls back to 1:1).
(int, int) _pos(ASTNode node) => (node.startLine ?? 1, node.startColumn ?? 1);

/// The nested-node children of [node], dropping bare punctuation
/// tokens.
List<ASTNode> _astChildren(ASTNode node) => [
      for (final c in node.children)
        if (c is ASTNode) c
    ];

/// Lower a parsed `program` CST node into the typed form sequence.
///
/// Expects `root.ruleName == "program"` (the grammar's start symbol).
///
/// Throws [McCarthyParseError] if the tree shape is unexpected
/// (should not happen for output of [parseMcCarthyToCst]) or an
/// integer literal overflows 64 bits. Rule names render
/// double-quoted, matching the reference's debug format on the simple
/// identifiers this grammar produces.
List<LispExpr> extractProgram(ASTNode root) {
  if (root.ruleName != 'program') {
    final (line, column) = _pos(root);
    throw McCarthyParseError(
      message: 'expected \'program\' root, got "${root.ruleName}"',
      line: line,
      column: column,
    );
  }
  return [for (final c in _astChildren(root)) _extractSexpr(c, 0)];
}

/// `sexpr = atom | list | quoted` — dispatch on the single child rule.
LispExpr _extractSexpr(ASTNode node, int depth) {
  final (line, column) = _pos(node);
  _checkDepth(depth, line, column);
  if (node.ruleName != 'sexpr') {
    throw McCarthyParseError(
      message: 'expected \'sexpr\', got "${node.ruleName}"',
      line: line,
      column: column,
    );
  }
  final kids = _astChildren(node);
  if (kids.isEmpty) {
    throw McCarthyParseError(
      message: 'empty sexpr node',
      line: line,
      column: column,
    );
  }
  final inner = kids.first;
  switch (inner.ruleName) {
    case 'atom':
      return _extractAtom(inner);
    case 'list':
      return _extractList(inner, depth + 1);
    case 'quoted':
      return _extractQuoted(inner, depth + 1);
    default:
      throw McCarthyParseError(
        message: 'unexpected sexpr child: "${inner.ruleName}"',
        line: line,
        column: column,
      );
  }
}

/// `atom = SYMBOL | INTEGER` — a single bare token.
LispExpr _extractAtom(ASTNode node) {
  final (line, column) = _pos(node);
  Token? found;
  for (final c in node.children) {
    if (c is Token) {
      found = c;
      break;
    }
  }
  if (found == null) {
    throw McCarthyParseError(
      message: 'empty atom node',
      line: line,
      column: column,
    );
  }
  final tok = found;
  switch (tok.type) {
    case 'SYMBOL':
      return LispExpr.sym(tok.value);
    case 'INTEGER':
      final value = int.tryParse(tok.value);
      if (value == null) {
        throw McCarthyParseError(
          message: 'integer literal "${tok.value}" does not fit in i64',
          line: tok.line,
          column: tok.column,
        );
      }
      return LispInt(value);
    default:
      throw McCarthyParseError(
        message:
            'unexpected atom token: type="${tok.type}" value="${tok.value}"',
        line: tok.line,
        column: tok.column,
      );
  }
}

/// `list = LPAREN list_body RPAREN`, where
/// `list_body = [ sexpr { sexpr } [ DOT sexpr ] ]`.
///
/// Builds the nested-`Cons` encoding. A list with no dotted tail is
/// `NIL`-terminated; an explicit `DOT` tail becomes the final `cdr`.
LispExpr _extractList(ASTNode node, int depth) {
  final (line, column) = _pos(node);
  _checkDepth(depth, line, column);

  // The single nested child is `list_body` (the LPAREN/RPAREN are bare
  // tokens, dropped by `_astChildren`). An empty list `()` may yield a
  // list_body node with no children — treat a missing/empty body as NIL.
  final kids = _astChildren(node);
  if (kids.isEmpty) {
    return const LispNil();
  }
  final body = kids.first;

  final elems = <LispExpr>[];
  LispExpr? tail;
  var afterDot = false;

  for (final child in body.children) {
    if (child is Token && child.type == 'DOT') {
      afterDot = true;
    } else if (child is ASTNode && child.ruleName == 'sexpr') {
      final expr = _extractSexpr(child, depth + 1);
      if (afterDot) {
        tail = expr;
      } else {
        elems.add(expr);
      }
    }
    // Any other token (none expected from this grammar) is ignored.
  }

  var acc = tail ?? const LispNil();
  for (var i = elems.length - 1; i >= 0; i--) {
    acc = LispCons(elems[i], acc);
  }
  return acc;
}

/// `quoted = QUOTE sexpr` — rewrite `'X` to `(QUOTE X)`.
LispExpr _extractQuoted(ASTNode node, int depth) {
  final (line, column) = _pos(node);
  _checkDepth(depth, line, column);
  final kids = _astChildren(node);
  if (kids.isEmpty) {
    throw McCarthyParseError(
      message: "expected an expression after '",
      line: line,
      column: column,
    );
  }
  return LispExpr.quote(_extractSexpr(kids.first, depth + 1));
}
