import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

List<Token> tokenizeJson(String source) =>
    grammarTokenize(source, tokenGrammar);
