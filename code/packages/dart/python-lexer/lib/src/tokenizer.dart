/// Python tokenization over the compiled versioned grammars:
/// legacy minor-line version resolution, the factory, the one-shot
/// entry point, and the LANG84 release tables. Token types keep their
/// grammar names verbatim; the final token is always `EOF`.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

export '_grammar.dart' show pythonGrammarClass, tokenGrammarForRelease;

/// The legacy minor-line versions the version API accepts, in
/// historical order.
const List<String> supportedPythonVersions = [
  '2.7',
  '3.0',
  '3.6',
  '3.8',
  '3.10',
  '3.12',
];

/// The legacy version an empty version string selects, mirroring the
/// reference's `DEFAULT_VERSION`.
const String defaultPythonVersion = '3.12';

/// The LANG84 paired releases, in inventory (date) order — not
/// version-sorted, on purpose.
const List<String> supportedPythonReleases = [
  '2.2',
  '2.4',
  '3.0a1',
  '2.6',
  '3.6.0',
  '2.7.18',
  '3.5.10',
  '3.12.3',
  '3.13.16',
  '3.11.17',
  '3.16.0a1',
];

/// Resolves [version] to a supported legacy version: empty means
/// [defaultPythonVersion], anything unsupported throws an
/// [ArgumentError] carrying the reference's exact message (the
/// reference returns `Err(String)` there instead). Release IDs are
/// not versions: asking for one here is an error (LANG84 C4).
String resolvePythonVersion(String version) {
  final resolved = version.isEmpty ? defaultPythonVersion : version;
  if (supportedPythonVersions.contains(resolved)) {
    return resolved;
  }
  final valid = supportedPythonVersions.map((v) => '"$v"').join(', ');
  throw ArgumentError(
    "Unsupported Python version '$version'. Supported versions: $valid",
  );
}

/// Tokenizes Python [source] under the legacy [version] grammar
/// (empty selects [defaultPythonVersion]) in one shot.
/// Throws [ArgumentError] for an unknown version and [LexerError] on
/// unrecognized input (the reference wraps the latter in a String;
/// Dart lanes propagate the structured error instead).
///
/// There is no `createPythonLexer` factory: every Python grammar runs
/// in indentation mode, which the engine serves only through the
/// [grammarTokenize] one-shot ([GrammarLexer] is standard-mode only).
/// The Dart toml-lexer lane makes the same cut for the same reason.
List<Token> tokenizePython(String source, [String version = '']) =>
    grammarTokenize(
        source, tokenGrammarForLegacyVersion(resolvePythonVersion(version)));
