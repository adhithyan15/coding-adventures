// ============================================================================
// CrossValidator.java — Cross-validates .tokens and .grammar files
// ============================================================================
//
// The .grammar file references tokens by UPPERCASE name. The .tokens file
// defines which tokens exist. This module checks that the two are consistent:
//
//   1. Every UPPERCASE name in the grammar must be defined in the tokens file.
//      If not, the parser will try to match a token type the lexer never emits.
//
//   2. Every token in the .tokens file should ideally be used somewhere in
//      the grammar. Unused tokens suggest typos or leftover cruft.
//
// Synthetic tokens (NEWLINE, INDENT, DEDENT, EOF) are always valid — the
// lexer produces these implicitly.
// ============================================================================

package com.codingadventures.grammartools;

import java.util.*;

/**
 * Cross-validates a {@link TokenGrammar} and {@link ParserGrammar} for consistency.
 */
public final class CrossValidator {

    private CrossValidator() {}

    /**
     * Check that a TokenGrammar and ParserGrammar are consistent.
     *
     * @return list of error/warning strings; empty means fully consistent
     */
    public static List<String> crossValidate(TokenGrammar tokenGrammar, ParserGrammar parserGrammar) {
        List<String> issues = new ArrayList<>();

        // Build the set of all defined token names (including aliases)
        Set<String> definedTokens = new HashSet<>(tokenGrammar.tokenNames());

        // Add synthetic tokens
        definedTokens.add("NEWLINE");
        definedTokens.add("EOF");
        if ("indentation".equals(tokenGrammar.getMode())) {
            definedTokens.add("INDENT");
            definedTokens.add("DEDENT");
        }

        Set<String> referencedTokens = parserGrammar.tokenReferences();

        // Missing token references (errors)
        for (String ref : sorted(referencedTokens)) {
            if (!definedTokens.contains(ref)) {
                issues.add("Error: Grammar references token '" + ref
                        + "' which is not defined in the tokens file");
            }
        }

        // Unused tokens (warnings)
        for (TokenDefinition defn : tokenGrammar.getDefinitions()) {
            boolean isUsed = referencedTokens.contains(defn.getName());
            if (defn.getAlias() != null && referencedTokens.contains(defn.getAlias())) {
                isUsed = true;
            }
            if (!isUsed) {
                issues.add("Warning: Token '" + defn.getName() + "' (line " + defn.getLineNumber()
                        + ") is defined but never used in the grammar");
            }
        }

        return issues;
    }

    private static List<String> sorted(Set<String> set) {
        List<String> list = new ArrayList<>(set);
        Collections.sort(list);
        return list;
    }
}
