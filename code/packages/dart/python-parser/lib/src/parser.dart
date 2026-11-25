/// Python parsing: the legacy minor-line API plus the LANG84
/// release frontend (budgets, typed errors, per-release pairs).
library;

import 'dart:convert';

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_python_lexer/python_lexer.dart'
    show
        defaultPythonVersion,
        pythonGrammarClass,
        tokenGrammarForRelease,
        tokenizePython;

import '_grammar.dart';

export '_grammar.dart'
    show
        pythonParserGrammarClass,
        releaseParserGrammar,
        supportedPythonParserReleases;

/// The legacy minor-line versions the version API accepts, in
/// historical order.
const List<String> supportedPythonParserVersions = [
  '2.7',
  '3.0',
  '3.6',
  '3.8',
  '3.10',
  '3.12',
];

/// Recursion-depth cap for the Python [GrammarParser], mirroring the
/// reference's measured `MAX_RULE_DEPTH` of 190: plain parenthesised
/// nesting parses cleanly to 9 levels (10 trips) — the one recursion
/// shape measured so far, on the newest embedded grammar.
/// (Named for the language because `FrontendBudgets` already owns
/// `maxRuleDepth` for its per-parse field, exactly like the
/// reference's `MAX_RULE_DEPTH` vs `max_rule_depth`.)
const int pythonMaxRuleDepth = 190;

/// Clamps a requested depth to the crate's measured safe cap.
int _clampDepth(int requested) =>
    requested < pythonMaxRuleDepth ? requested : pythonMaxRuleDepth;

/// Byte cap on the caller-supplied version echoed into the
/// [ArgumentError] message: real versions are a few bytes, so
/// anything longer is truncated (with `...`) after [_echo]
/// neutralization, exactly like the release frontend treats
/// over-long IDs. Printable short versions — every input the
/// reference's message is mirrored against — render verbatim, so
/// the reference-exact message is preserved on the mirrored domain
/// while hostile versions can neither forge log lines nor amplify
/// the log.
const int maxVersionEchoBytes = 32;

/// Renders [version] for the unknown-version error: verbatim when it
/// is short and printable (the reference-mirrored domain),
/// [_echo]-neutralized and truncated otherwise.
String _versionEcho(String version) {
  if (_utf8Length(version) > maxVersionEchoBytes) {
    return '${_echo(_truncate(version, maxVersionEchoBytes))}...';
  }
  return _echo(version);
}

/// Resolves [version] to a supported legacy version: empty means the
/// lexer's [defaultPythonVersion], anything unsupported throws an
/// [ArgumentError] carrying the reference's exact message for
/// printable short versions (the reference returns `Err(String)`
/// there instead); hostile versions are [_echo]-neutralized and
/// capped at [maxVersionEchoBytes] so the echo cannot forge log
/// lines or amplify the log. Release IDs are not versions: asking
/// for one here is an error.
String resolvePythonParserVersion(String version) {
  final resolved = version.isEmpty ? defaultPythonVersion : version;
  if (supportedPythonParserVersions.contains(resolved)) {
    return resolved;
  }
  final valid = supportedPythonParserVersions.map((v) => '"$v"').join(', ');
  throw ArgumentError(
    "Unsupported Python version '${_versionEcho(version)}'. "
    'Supported versions: $valid',
  );
}

/// Creates a [GrammarParser] for [source] under the legacy [version]
/// grammar (empty selects [defaultPythonParserVersion]), with the
/// recursion-depth guard enabled. Trusted input only: unlike the
/// release frontend, the legacy entry points enforce no source-byte
/// or token-count budget, so untrusted callers should use
/// [parsePythonAt] with [FrontendBudgets] instead.
/// Throws [ArgumentError] for an unknown version and [LexerError] on
/// unrecognized input.
GrammarParser createPythonParser(String source, [String version = '']) {
  final resolved = resolvePythonParserVersion(version);
  final tokens = tokenizePython(source, resolved);
  return GrammarParser(
    tokens,
    legacyParserGrammar(resolved),
    options: const GrammarParserOptions(maxDepth: pythonMaxRuleDepth),
  );
}

/// Parses Python [source] under the legacy [version] grammar (empty
/// selects the default) into an [ASTNode] rooted at
/// the grammar's start rule (`file`, except 3.8's `file_input`).
/// Trusted input only (see [createPythonParser]): untrusted callers
/// should use [parsePythonAt] with [FrontendBudgets].
/// Throws [ArgumentError] for an unknown version, [LexerError] on
/// unrecognized input, and [GrammarParseError] on grammar violations
/// (the reference wraps the latter in a String; Dart lanes propagate
/// the structured error instead).
ASTNode parsePython(String source, [String version = '']) =>
    createPythonParser(source, version).parse();

/// Parses Python [source], returning `null` instead of throwing on
/// any failure. Mirrors the reference's `Result`-returning
/// `parse_python` with idiomatic Dart nullability.
ASTNode? tryParsePython(String source, [String version = '']) {
  try {
    return parsePython(source, version);
  } on ArgumentError {
    return null;
  } on LexerError {
    return null;
  } on GrammarParseError {
    return null;
  }
}

// ---------------------------------------------------------------------------
// LANG84 release frontend
// ---------------------------------------------------------------------------

/// The release used when a caller names none. It is an exact
/// inventory ID (the host `python3` behind the phase-7a oracle).
const String defaultPythonRelease = '3.13.16';

/// Default source budget for untrusted programs (LANG84 C5).
const int defaultMaxSourceBytes = 64 * 1024;

/// Ceiling a trusted caller may raise the source budget to.
const int maxTrustedSourceBytes = 1024 * 1024;

/// Default token budget for untrusted programs, checked after lexing
/// and before the parser builds its memo.
const int defaultMaxTokens = 16 * 1024;

/// Ceiling a trusted caller may raise the token budget to.
const int maxTrustedTokens = 256 * 1024;

/// Upper bound for one release ID; longer requests are rejected
/// before any lookup so a hostile string cannot be echoed back at
/// length.
const int maxReleaseIdBytes = 64;

/// Limits applied to one parse. The default constructor is the
/// untrusted profile.
class FrontendBudgets {
  const FrontendBudgets({
    this.maxSourceBytes = defaultMaxSourceBytes,
    this.maxRuleDepth = pythonMaxRuleDepth,
    this.maxTokens = defaultMaxTokens,
  });

  /// Source size in bytes, checked before lexing. Values above
  /// [maxTrustedSourceBytes] are clamped to it.
  final int maxSourceBytes;

  /// Parser recursion cap. Values above the crate's measured safe cap
  /// ([maxRuleDepth]) are clamped to it: a deeper cap could overflow
  /// the native stack, which is an abort, not an error.
  final int maxRuleDepth;

  /// Token count, checked after lexing and before parsing. Values
  /// above [maxTrustedTokens] are clamped to it.
  final int maxTokens;

  int get sourceLimit => maxSourceBytes < maxTrustedSourceBytes
      ? maxSourceBytes
      : maxTrustedSourceBytes;

  int get tokenLimit =>
      maxTokens < maxTrustedTokens ? maxTokens : maxTrustedTokens;

  int get depthLimit => _clampDepth(maxRuleDepth);

  @override
  bool operator ==(Object other) =>
      other is FrontendBudgets &&
      other.maxSourceBytes == maxSourceBytes &&
      other.maxRuleDepth == maxRuleDepth &&
      other.maxTokens == maxTokens;

  @override
  int get hashCode => Object.hash(maxSourceBytes, maxRuleDepth, maxTokens);
}

/// Every way a release-selected parse can fail. These are the LANG84
/// C6 categories that belong to the parser.
sealed class PythonFrontendError implements Exception {
  const PythonFrontendError();
}

/// The ID is not a row of `code/grammars/python/releases.csv`.
final class PythonUnknownRelease extends PythonFrontendError {
  const PythonUnknownRelease(
      {required this.requested, required this.suggestions});

  /// The request, truncated and with control characters escaped.
  final String requested;

  /// Up to five inventory IDs sharing the longest prefix.
  final List<String> suggestions;

  @override
  bool operator ==(Object other) =>
      other is PythonUnknownRelease &&
      other.requested == requested &&
      _stringListsEqual(other.suggestions, suggestions);

  @override
  int get hashCode => Object.hash(requested, Object.hashAll(suggestions));

  @override
  String toString() {
    final buf = StringBuffer('unknown Python release "$requested"');
    if (suggestions.isNotEmpty) {
      buf.write(' (did you mean ${suggestions.join(', ')}?)');
    }
    return buf.toString();
  }
}

/// The row exists but has no pair yet.
final class PythonReleaseNotAvailable extends PythonFrontendError {
  const PythonReleaseNotAvailable({required this.release, required this.stage});

  /// The inventory ID.
  final String release;

  /// The row's `grammar_stage`, or why its table is missing.
  final String stage;

  @override
  bool operator ==(Object other) =>
      other is PythonReleaseNotAvailable &&
      other.release == release &&
      other.stage == stage;

  @override
  int get hashCode => Object.hash(release, stage);

  @override
  String toString() =>
      'Python $release has no grammar pair yet (grammar_stage $stage)';
}

/// A C5 budget was exceeded.
final class PythonBudgetExceeded extends PythonFrontendError {
  const PythonBudgetExceeded(
      {required this.release, required this.which, required this.limit});

  /// The release the parse ran against.
  final String release;

  /// Which budget (`source bytes`, `tokens`, `parser rule depth`).
  final String which;

  /// Its limit.
  final int limit;

  @override
  bool operator ==(Object other) =>
      other is PythonBudgetExceeded &&
      other.release == release &&
      other.which == which &&
      other.limit == limit;

  @override
  int get hashCode => Object.hash(release, which, limit);

  @override
  String toString() => 'Python $release: $which exceeds the limit of $limit';
}

/// No token pattern of the release's `.tokens` matches here.
final class PythonLexError extends PythonFrontendError {
  const PythonLexError(
      {required this.release,
      required this.line,
      required this.column,
      required this.message});

  /// The release the parse ran against.
  final String release;

  /// 1-based line.
  final int line;

  /// 1-based column.
  final int column;

  /// The lexer's message.
  final String message;

  @override
  bool operator ==(Object other) =>
      other is PythonLexError &&
      other.release == release &&
      other.line == line &&
      other.column == column &&
      other.message == message;

  @override
  int get hashCode => Object.hash(release, line, column, message);

  @override
  String toString() => 'Python $release lex error at $line:$column: $message';
}

/// The release's `.grammar` rejects the token stream.
final class PythonParseError extends PythonFrontendError {
  const PythonParseError(
      {required this.release,
      required this.line,
      required this.column,
      required this.message});

  /// The release the parse ran against.
  final String release;

  /// 1-based line of the furthest failure.
  final int line;

  /// 1-based column of the furthest failure.
  final int column;

  /// The parser's message, listing the expected terminals.
  final String message;

  @override
  bool operator ==(Object other) =>
      other is PythonParseError &&
      other.release == release &&
      other.line == line &&
      other.column == column &&
      other.message == message;

  @override
  int get hashCode => Object.hash(release, line, column, message);

  @override
  String toString() => 'Python $release parse error at $line:$column: $message';
}

bool _stringListsEqual(List<String> a, List<String> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

/// A parse tree together with the release that produced it.
class PythonParse {
  const PythonParse({required this.release, required this.ast});

  /// The exact inventory ID whose pair parsed the source.
  final String release;

  /// The `file` node of the release's grammar.
  final ASTNode ast;
}

/// The deduplicated table classes `(tokens, grammar)` a release
/// compiles to, or `null` for a release without tables. Releases
/// whose files differ only in comments share a class.
(String, String)? pythonGrammarClasses(String release) {
  final tokens = pythonGrammarClass(release);
  final grammar = pythonParserGrammarClass(release);
  if (tokens == null || grammar == null) {
    return null;
  }
  return (tokens, grammar);
}

/// Resolves a request to the exact inventory ID whose pair will be
/// used: `null` means [defaultPythonRelease]; anything else must be
/// exact (no fallback, no neighbours). Throws [PythonFrontendError].
/// UTF-8 length with lone surrogates counted as U+FFFD (3 bytes),
/// so hostile strings cannot crash the measurement itself
/// (`utf8.encode` throws on them).
int _utf8Length(String text) {
  var bytes = 0;
  for (final rune in text.runes) {
    if (rune >= 0xD800 && rune <= 0xDFFF) {
      bytes += 3;
    } else if (rune < 0x80) {
      bytes += 1;
    } else if (rune < 0x800) {
      bytes += 2;
    } else if (rune < 0x10000) {
      bytes += 3;
    } else {
      bytes += 4;
    }
  }
  return bytes;
}

String resolvePythonRelease(String? release) {
  final requested = release ?? defaultPythonRelease;
  if (_utf8Length(requested) > maxReleaseIdBytes) {
    throw PythonUnknownRelease(
        requested: '${_echo(_truncate(requested, maxReleaseIdBytes))}...',
        suggestions: const []);
  }
  if (supportedPythonParserReleases.contains(requested)) {
    return requested;
  }
  final stage = pythonReleaseStage(requested);
  if (stage != null) {
    throw PythonReleaseNotAvailable(release: requested, stage: stage);
  }
  throw PythonUnknownRelease(
      requested: _echo(requested), suggestions: _suggestions(requested));
}

/// Lexes [source] with exactly one release's `.tokens` table.
/// Returns the resolved release ID plus the token stream.
/// Throws [PythonFrontendError] (unknown release, missing pair,
/// budget, or lex failure).
(String, List<Token>) tokenizePythonAt(
    String source, String? release, FrontendBudgets budgets) {
  final resolved = resolvePythonRelease(release);
  final limit = budgets.sourceLimit;
  if (_utf8Length(source) > limit) {
    throw PythonBudgetExceeded(
        release: resolved, which: 'source bytes', limit: limit);
  }
  // Both tables generate from the same CSV rows. A release present
  // in one and not the other is a regeneration bug, reported rather
  // than papered over.
  final grammar = tokenGrammarForRelease(resolved);
  if (grammar == null) {
    // coverage:ignore-start
    // Unreachable in a correct tree: every paired release ships both
    // tables (proven by the per-release probe tests). Kept to report
    // a regeneration bug instead of papering over it.
    throw PythonReleaseNotAvailable(
        release: resolved,
        stage: 'token table missing (regenerate python-lexer)');
    // coverage:ignore-end
  }
  late final List<Token> tokens;
  try {
    tokens = grammarTokenize(source, grammar);
  } on LexerError catch (e) {
    throw PythonLexError(
        release: resolved, line: e.line, column: e.column, message: e.message);
  }
  final tokenLimit = budgets.tokenLimit;
  if (tokens.length > tokenLimit) {
    throw PythonBudgetExceeded(
        release: resolved, which: 'tokens', limit: tokenLimit);
  }
  return (resolved, tokens);
}

/// Lexes and parses [source] with exactly one release's pair.
/// Throws [PythonFrontendError].
PythonParse parsePythonAt(
    String source, String? release, FrontendBudgets budgets) {
  final (resolved, tokens) = tokenizePythonAt(source, release, budgets);
  final grammar = releaseParserGrammar(resolved);
  if (grammar == null) {
    // coverage:ignore-start
    // Unreachable in a correct tree: every paired release ships both
    // tables (proven by the per-release probe tests). Kept to report
    // a regeneration bug instead of papering over it.
    throw PythonReleaseNotAvailable(
        release: resolved,
        stage: 'parser table missing (regenerate python-parser)');
    // coverage:ignore-end
  }
  final depth = budgets.depthLimit;
  late final ASTNode ast;
  try {
    ast = GrammarParser(tokens, grammar,
            options: GrammarParserOptions(maxDepth: depth))
        .parse();
  } on GrammarParseError catch (e) {
    // The parser reports its depth guard as an ordinary error: either
    // its dedicated message, or (when an enclosing optional or
    // repetition backtracked past the refusal) a furthest-failure
    // message whose expected terminal is the guard's marker. Both
    // are the budget, not a syntax error.
    if (e.message.startsWith('input nests deeper than the supported limit') ||
        e.message.contains('input within the supported nesting limit')) {
      throw PythonBudgetExceeded(
          release: resolved, which: 'parser rule depth', limit: depth);
    }
    // The token is absent only for the engine-internal empty-grammar
    // error, unreachable with compiled grammars; default to 1:1.
    final token = e.token;
    throw PythonParseError(
        release: resolved,
        line: token?.line ?? 1,
        column: token?.column ?? 1,
        message: e.message);
  }
  return PythonParse(release: resolved, ast: ast);
}

/// Up to five inventory IDs sharing the longest byte-prefix with
/// [requested] (inventory order). Empty when nothing shares a byte.
List<String> _suggestions(String requested) {
  int common(String id) {
    final a = _utf8Bytes(id);
    final b = _utf8Bytes(requested);
    var n = 0;
    while (n < a.length && n < b.length && a[n] == b[n]) {
      n++;
    }
    return n;
  }

  var best = 0;
  for (final id in pythonInventoryIds) {
    final n = common(id);
    if (n > best) {
      best = n;
    }
  }
  if (best == 0) {
    return const [];
  }
  final out = <String>[];
  for (final id in pythonInventoryIds) {
    if (common(id) == best) {
      out.add(id);
      if (out.length == 5) {
        break;
      }
    }
  }
  return out;
}

/// Neutralizes caller-supplied text before it enters a diagnostic: a
/// raw newline would forge a log line, an ESC would drive the
/// terminal. Mirrors Rust's `char::escape_debug` over its testified
/// contract: printable text passes through; `\t`/`\r`/`\n` take
/// short escapes; quotes and backslashes are backslashed; other
/// control characters (C0, DEL, C1) and lone surrogates render as
/// `\u{h...}` (lowercase, minimal digits).
///
/// Deliberate divergence, probed against `rustc`: Rust also escapes
/// non-control format/marks (Cf, Mn, Me, Zl/Zp, NBSP, unassigned and
/// private-use code points), which this port passes through. No test
/// pins those cases, and both spellings satisfy the security
/// contract (no control character ever reaches the output, so no log
/// line can be forged and no terminal sequence smuggled).
String _echo(String text) {
  final out = StringBuffer();
  for (final rune in text.runes) {
    switch (rune) {
      case 0x09:
        out.write(r'\t');
      case 0x0D:
        out.write(r'\r');
      case 0x0A:
        out.write(r'\n');
      case 0x5C:
        out.write(r'\\');
      case 0x27:
        out.write(r"\'");
      case 0x22:
        out.write(r'\"');
      default:
        if ((rune >= 0x00 && rune <= 0x1F) ||
            (rune >= 0x7F && rune <= 0x9F) ||
            (rune >= 0xD800 && rune <= 0xDFFF)) {
          out.write('\\u{${rune.toRadixString(16)}}');
        } else {
          out.writeCharCode(rune);
        }
    }
  }
  return out.toString();
}

/// UTF-8 bytes with lone surrogates substituted as U+FFFD, so
/// hostile strings cannot crash the measurement itself
/// (`utf8.encode` throws on them).
List<int> _utf8Bytes(String text) {
  if (!_hasSurrogates(text)) {
    return utf8.encode(text);
  }
  return utf8.encode(String.fromCharCodes(text.runes
      .map((rune) => rune >= 0xD800 && rune <= 0xDFFF ? 0xFFFD : rune)));
}

bool _hasSurrogates(String text) {
  for (var i = 0; i < text.length; i++) {
    final unit = text.codeUnitAt(i);
    if (unit >= 0xD800 && unit <= 0xDFFF) {
      return true;
    }
  }
  return false;
}

/// Truncates [text] to [max] UTF-8 bytes at a rune boundary.
String _truncate(String text, int max) {
  var bytes = 0;
  final out = StringBuffer();
  for (final rune in text.runes) {
    final width = rune < 0x80
        ? 1
        : rune < 0x800
            ? 2
            : rune < 0x10000
                ? 3
                : 4;
    if (bytes + width > max) {
      break;
    }
    bytes += width;
    out.writeCharCode(rune);
  }
  return out.toString();
}
