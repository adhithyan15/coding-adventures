// ============================================================================
// TokenGrammarValidator.java — Lint pass for parsed TokenGrammar
// ============================================================================
//
// This validates a TokenGrammar that was already parsed successfully. We look
// for semantic issues: duplicate names, invalid regex patterns, naming
// convention violations, invalid modes, empty groups.
// ============================================================================

package com.codingadventures.grammartools;

import java.util.*;
import java.util.regex.Pattern;
import java.util.regex.PatternSyntaxException;

/**
 * Validates a parsed {@link TokenGrammar} for common semantic problems.
 */
public final class TokenGrammarValidator {

    private static final Pattern GROUP_NAME_PATTERN = Pattern.compile("^[a-z_][a-z0-9_]*$");

    private TokenGrammarValidator() {}

    /**
     * Check a parsed TokenGrammar for issues.
     *
     * @return a list of warning/error strings; empty means no issues
     */
    public static List<String> validate(TokenGrammar grammar) {
        List<String> issues = new ArrayList<>();

        issues.addAll(validateDefinitions(grammar.getDefinitions(), "token"));
        issues.addAll(validateDefinitions(grammar.getSkipDefinitions(), "skip pattern"));
        issues.addAll(validateDefinitions(grammar.getErrorDefinitions(), "error pattern"));

        // Validate mode
        if (grammar.getMode() != null && !"indentation".equals(grammar.getMode())) {
            issues.add("Unknown lexer mode '" + grammar.getMode() + "' (only 'indentation' is supported)");
        }

        // Validate escape mode
        if (grammar.getEscapeMode() != null && !"none".equals(grammar.getEscapeMode())) {
            issues.add("Unknown escape mode '" + grammar.getEscapeMode() + "' (only 'none' is supported)");
        }

        // Validate pattern groups
        for (var entry : grammar.getGroups().entrySet()) {
            String groupName = entry.getKey();
            PatternGroup group = entry.getValue();

            if (!GROUP_NAME_PATTERN.matcher(groupName).matches()) {
                issues.add("Invalid group name '" + groupName + "' (must be a lowercase identifier)");
            }
            if (group.getDefinitions().isEmpty()) {
                issues.add("Empty pattern group '" + groupName + "' (has no token definitions)");
            }
            issues.addAll(validateDefinitions(group.getDefinitions(), "group '" + groupName + "' token"));
        }

        return issues;
    }

    private static List<String> validateDefinitions(List<TokenDefinition> definitions, String label) {
        List<String> issues = new ArrayList<>();
        Map<String, Integer> seenNames = new HashMap<>();

        for (TokenDefinition defn : definitions) {
            // Duplicate check
            if (seenNames.containsKey(defn.getName())) {
                issues.add("Line " + defn.getLineNumber() + ": Duplicate " + label + " name '"
                        + defn.getName() + "' (first defined on line " + seenNames.get(defn.getName()) + ")");
            } else {
                seenNames.put(defn.getName(), defn.getLineNumber());
            }

            // Empty pattern check
            if (defn.getPattern().isEmpty()) {
                issues.add("Line " + defn.getLineNumber() + ": Empty pattern for " + label + " '" + defn.getName() + "'");
            }

            // Invalid regex check
            if (defn.isRegex()) {
                try {
                    Pattern.compile(defn.getPattern());
                } catch (PatternSyntaxException e) {
                    issues.add("Line " + defn.getLineNumber() + ": Invalid regex for " + label + " '"
                            + defn.getName() + "': " + e.getDescription());
                }
            }

            // Naming convention: UPPER_CASE
            if (!defn.getName().equals(defn.getName().toUpperCase())) {
                issues.add("Line " + defn.getLineNumber() + ": Token name '" + defn.getName() + "' should be UPPER_CASE");
            }

            // Alias naming convention
            if (defn.getAlias() != null && !defn.getAlias().equals(defn.getAlias().toUpperCase())) {
                issues.add("Line " + defn.getLineNumber() + ": Alias '" + defn.getAlias()
                        + "' for token '" + defn.getName() + "' should be UPPER_CASE");
            }
        }

        return issues;
    }
}
