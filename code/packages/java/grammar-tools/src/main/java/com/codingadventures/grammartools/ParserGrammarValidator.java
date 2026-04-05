// ============================================================================
// ParserGrammarValidator.java — Lint pass for parsed ParserGrammar
// ============================================================================

package com.codingadventures.grammartools;

import java.util.*;

/**
 * Validates a parsed {@link ParserGrammar} for semantic issues.
 */
public final class ParserGrammarValidator {

    /** Synthetic tokens always valid — the lexer produces these implicitly. */
    private static final Set<String> SYNTHETIC_TOKENS = Set.of("NEWLINE", "INDENT", "DEDENT", "EOF");

    private ParserGrammarValidator() {}

    /**
     * Check a parsed ParserGrammar for issues.
     *
     * @param grammar    the grammar to validate
     * @param tokenNames optional set of valid token names from a .tokens file; null to skip token checks
     * @return list of issue strings; empty means no issues
     */
    public static List<String> validate(ParserGrammar grammar, Set<String> tokenNames) {
        List<String> issues = new ArrayList<>();

        Set<String> defined = grammar.ruleNames();
        Set<String> referencedRules = grammar.ruleReferences();
        Set<String> referencedTokens = grammar.tokenReferences();

        // Duplicate rule names
        Map<String, Integer> seen = new HashMap<>();
        for (GrammarRule rule : grammar.getRules()) {
            if (seen.containsKey(rule.name())) {
                issues.add("Line " + rule.lineNumber() + ": Duplicate rule name '" + rule.name()
                        + "' (first defined on line " + seen.get(rule.name()) + ")");
            } else {
                seen.put(rule.name(), rule.lineNumber());
            }
        }

        // Non-lowercase rule names
        for (GrammarRule rule : grammar.getRules()) {
            if (!rule.name().equals(rule.name().toLowerCase())) {
                issues.add("Line " + rule.lineNumber() + ": Rule name '" + rule.name() + "' should be lowercase");
            }
        }

        // Undefined rule references
        for (String ref : sorted(referencedRules)) {
            if (!defined.contains(ref)) {
                issues.add("Undefined rule reference: '" + ref + "'");
            }
        }

        // Undefined token references
        if (tokenNames != null) {
            for (String ref : sorted(referencedTokens)) {
                if (!tokenNames.contains(ref) && !SYNTHETIC_TOKENS.contains(ref)) {
                    issues.add("Undefined token reference: '" + ref + "'");
                }
            }
        }

        // Unreachable rules
        if (!grammar.getRules().isEmpty()) {
            String startRule = grammar.getRules().get(0).name();
            for (GrammarRule rule : grammar.getRules()) {
                if (!rule.name().equals(startRule) && !referencedRules.contains(rule.name())) {
                    issues.add("Line " + rule.lineNumber() + ": Rule '" + rule.name()
                            + "' is defined but never referenced (unreachable)");
                }
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
