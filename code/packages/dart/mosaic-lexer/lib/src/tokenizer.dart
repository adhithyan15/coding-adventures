import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Tokenizes Mosaic component source using the embedded canonical grammar.
List<Token> tokenizeMosaic(String source) =>
    grammarTokenize(source, tokenGrammar);
