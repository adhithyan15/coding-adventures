import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Tokenizes TOML source using the embedded canonical token grammar.
List<Token> tokenizeToml(String source) =>
    grammarTokenize(source, tokenGrammar);
