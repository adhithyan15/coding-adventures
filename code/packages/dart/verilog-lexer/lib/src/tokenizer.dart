/// Tokenizer implementation and version resolver for Verilog source text.
library;

import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart' as vgeneric;
import '_grammar_1995.dart' as v1995;
import '_grammar_2001.dart' as v2001;
import '_grammar_2005.dart' as v2005;
import 'constants.dart';
import 'errors.dart';
import 'preprocessor.dart';

/// Embedded pre-compiled token grammars keyed by canonical edition.
final Map<String, TokenGrammar> _grammars = <String, TokenGrammar>{
  'generic': vgeneric.tokenGrammar,
  '1995': v1995.tokenGrammar,
  '2001': v2001.tokenGrammar,
  '2005': v2005.tokenGrammar,
};

/// Resolves a caller-supplied version string or alias to a canonical edition identifier.
///
/// Throws [InvalidVerilogVersionException] if the version string is unrecognized.
String resolveVerilogVersion([String? version]) {
  if (version == null) {
    return defaultVerilogVersion;
  }
  final normalized = version.trim().toLowerCase();
  final resolved = verilogVersionAliases[normalized];
  if (resolved == null) {
    throw const InvalidVerilogVersionException();
  }
  return resolved;
}

/// Loads the precompiled [TokenGrammar] corresponding to the requested Verilog edition.
TokenGrammar loadVerilogTokenGrammar([String? version]) {
  final edition = resolveVerilogVersion(version);
  final grammar = _grammars[edition];
  if (grammar == null) {
    throw const InvalidVerilogVersionException();
  }
  return grammar;
}

/// A grammar-driven lexical analyzer for Verilog source.
final class VerilogLexer {
  /// Constructs a [VerilogLexer] for [source] text.
  VerilogLexer(
    this.source, {
    String? version,
    this.preprocess = true,
    this.predefined,
  }) : version = _initLexer(source, version);

  static String _initLexer(String source, String? version) {
    validateVerilogSource(source);
    return resolveVerilogVersion(version);
  }

  /// The input Verilog source code.
  final String source;

  /// The canonical resolved Verilog edition.
  final String version;

  /// Whether to run macro preprocessing before tokenization.
  final bool preprocess;

  /// Predefined macros to inject into the preprocessor.
  final Map<String, String>? predefined;

  /// Tokenizes the source text and returns the token stream.
  List<Token> tokenize() => tokenizeVerilog(
        source,
        version: version,
        preprocess: preprocess,
        predefined: predefined,
      );
}

/// Creates a [VerilogLexer] instance for [source].
VerilogLexer createVerilogLexer(
  String source, {
  String? version,
  bool preprocess = true,
  Map<String, String>? predefined,
}) =>
    VerilogLexer(
      source,
      version: version,
      preprocess: preprocess,
      predefined: predefined,
    );

/// Creates a [VerilogLexer] instance for raw [source] without preprocessing.
VerilogLexer createVerilogLexerRaw(String source, {String? version}) =>
    VerilogLexer(source, version: version, preprocess: false);

/// Tokenizes Verilog source code into a list of [Token] objects.
///
/// Runs the in-memory preprocessor by default prior to lexical tokenization.
List<Token> tokenizeVerilog(
  String source, {
  String? version,
  bool preprocess = true,
  Map<String, String>? predefined,
}) {
  validateVerilogSource(source);
  final processed =
      preprocess ? verilogPreprocess(source, predefined: predefined) : source;
  final grammar = loadVerilogTokenGrammar(version);
  try {
    return List<Token>.unmodifiable(grammarTokenize(processed, grammar));
  } on LexerError catch (e) {
    throw VerilogLexicalException(line: e.line, column: e.column);
  }
}

/// Tokenizes Verilog source directly without invoking the macro preprocessor.
List<Token> tokenizeVerilogRaw(String source, {String? version}) =>
    tokenizeVerilog(source, version: version, preprocess: false);
