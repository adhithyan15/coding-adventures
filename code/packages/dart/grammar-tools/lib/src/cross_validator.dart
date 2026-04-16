import 'parser_grammar.dart';
import 'token_grammar.dart';

List<String> crossValidate(
  TokenGrammar tokenGrammar,
  ParserGrammar parserGrammar,
) {
  final issues = <String>[];
  final definedTokens = tokenGrammar.tokenNames();

  if (tokenGrammar.mode == 'indentation') {
    definedTokens.addAll(const {'INDENT', 'DEDENT', 'NEWLINE'});
  }
  definedTokens.addAll(const {'NEWLINE', 'EOF'});

  final referencedTokens = parserGrammar.tokenReferences();
  for (final reference in referencedTokens.toList()..sort()) {
    if (!definedTokens.contains(reference)) {
      issues.add(
        "Error: Grammar references token '$reference' which is not defined in the tokens file",
      );
    }
  }

  for (final definition in tokenGrammar.definitions) {
    var isUsed = referencedTokens.contains(definition.name);
    final alias = definition.alias;
    if (alias != null && referencedTokens.contains(alias)) {
      isUsed = true;
    }

    if (!isUsed) {
      issues.add(
        "Warning: Token '${definition.name}' (line ${definition.lineNumber}) is defined but never used in the grammar",
      );
    }
  }

  return issues;
}
