// ============================================================================
// TokenGrammarParser.java — Parser for .tokens files
// ============================================================================
//
// A .tokens file describes the lexical grammar of a programming language:
// which patterns the lexer should recognize, in what order, and how to
// classify the matched text.
//
// The parser operates line-by-line with several modes:
//
//   1. Definition mode (default) — each line is a comment, blank, section
//      header, or token definition (NAME = /pattern/ or NAME = "literal").
//
//   2. Keywords mode — entered on "keywords:". Indented lines are keywords.
//
//   3. Reserved mode — entered on "reserved:". Same format as keywords but
//      populates reserved_keywords (which cause lex errors).
//
//   4. Skip mode — entered on "skip:". Indented lines define patterns that
//      the lexer matches and consumes silently (no token produced).
//
//   5. Errors mode — entered on "errors:". Error recovery patterns tried
//      when no normal token matches.
//
//   6. Group mode — entered on "group NAME:". Indented lines define token
//      patterns for context-sensitive lexing groups.
//
// Magic comments (# @key value) configure metadata like version and
// case-insensitivity. Unknown keys are silently ignored for forward compat.
//
// Layer: TE (text/language layer)
// ============================================================================

package com.codingadventures.grammartools;

import java.util.ArrayList;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Parses .tokens file text into a {@link TokenGrammar}.
 */
public final class TokenGrammarParser {

    // Magic comment pattern: # @key value
    private static final Pattern MAGIC_COMMENT = Pattern.compile("^#\\s*@(\\w+)\\s*(.*)$");

    // Valid group name: lowercase identifier
    private static final Pattern GROUP_NAME_PATTERN = Pattern.compile("^[a-z_][a-z0-9_]*$");

    // Valid token name: identifier (letters, digits, underscore)
    private static final Pattern TOKEN_NAME_PATTERN = Pattern.compile("^[a-zA-Z_][a-zA-Z0-9_]*$");

    // Reserved group names that cannot be used as pattern groups
    private static final java.util.Set<String> RESERVED_GROUP_NAMES = java.util.Set.of(
            "default", "skip", "keywords", "reserved", "errors", "layout_keywords", "context_keywords"
    );

    private TokenGrammarParser() {} // utility class

    /**
     * Parse the full text of a .tokens file into a TokenGrammar.
     *
     * @param source the complete text content of a .tokens file
     * @return a populated TokenGrammar
     * @throws TokenGrammarError if any line cannot be parsed
     */
    public static TokenGrammar parse(String source) throws TokenGrammarError {
        TokenGrammar grammar = new TokenGrammar();
        String[] lines = source.split("\n", -1);
        String currentSection = null;

        for (int i = 0; i < lines.length; i++) {
            int lineNumber = i + 1;
            String line = stripTrailing(lines[i]);
            String stripped = line.strip();

            // --- Blank lines ---
            if (stripped.isEmpty()) continue;

            // --- Comments and magic comments ---
            if (stripped.startsWith("#")) {
                Matcher m = MAGIC_COMMENT.matcher(stripped);
                if (m.matches()) {
                    String key = m.group(1);
                    String value = m.group(2).strip();
                    switch (key) {
                        case "version":
                            try { grammar.setVersion(Integer.parseInt(value)); }
                            catch (NumberFormatException ignored) {}
                            break;
                        case "case_insensitive":
                            grammar.setCaseInsensitive("true".equals(value));
                            break;
                        // Unknown keys silently ignored for forward compatibility
                    }
                }
                continue;
            }

            // --- mode: directive ---
            if (stripped.startsWith("mode:")) {
                String modeValue = stripped.substring(5).strip();
                if (modeValue.isEmpty()) {
                    throw new TokenGrammarError("Missing value after 'mode:'", lineNumber);
                }
                grammar.setMode(modeValue);
                currentSection = null;
                continue;
            }

            // --- escapes: directive ---
            if (stripped.startsWith("escapes:")) {
                String escapeValue = stripped.substring(8).strip();
                if (escapeValue.isEmpty()) {
                    throw new TokenGrammarError("Missing value after 'escapes:'", lineNumber);
                }
                grammar.setEscapeMode(escapeValue);
                currentSection = null;
                continue;
            }

            // --- case_sensitive: directive ---
            if (stripped.startsWith("case_sensitive:")) {
                String csValue = stripped.substring(15).strip().toLowerCase();
                if (!"true".equals(csValue) && !"false".equals(csValue)) {
                    throw new TokenGrammarError(
                            "Invalid value for 'case_sensitive:': '" + csValue + "' (expected 'true' or 'false')",
                            lineNumber);
                }
                grammar.setCaseSensitive("true".equals(csValue));
                currentSection = null;
                continue;
            }

            // --- Group headers: "group NAME:" ---
            if (stripped.startsWith("group ") && stripped.endsWith(":")) {
                String groupName = stripped.substring(6, stripped.length() - 1).strip();
                if (groupName.isEmpty()) {
                    throw new TokenGrammarError("Missing group name after 'group'", lineNumber);
                }
                if (!GROUP_NAME_PATTERN.matcher(groupName).matches()) {
                    throw new TokenGrammarError(
                            "Invalid group name: '" + groupName + "' (must be a lowercase identifier)",
                            lineNumber);
                }
                if (RESERVED_GROUP_NAMES.contains(groupName)) {
                    throw new TokenGrammarError(
                            "Reserved group name: '" + groupName + "'",
                            lineNumber);
                }
                if (grammar.getGroups().containsKey(groupName)) {
                    throw new TokenGrammarError("Duplicate group name: '" + groupName + "'", lineNumber);
                }
                grammar.getGroups().put(groupName, new PatternGroup(groupName, new ArrayList<>()));
                currentSection = "group:" + groupName;
                continue;
            }

            // --- Section headers ---
            if ("keywords:".equals(stripped) || "keywords :".equals(stripped)) {
                currentSection = "keywords"; continue;
            }
            if ("reserved:".equals(stripped) || "reserved :".equals(stripped)) {
                currentSection = "reserved"; continue;
            }
            if ("skip:".equals(stripped) || "skip :".equals(stripped)) {
                currentSection = "skip"; continue;
            }
            if ("errors:".equals(stripped) || "errors :".equals(stripped)) {
                currentSection = "errors"; continue;
            }
            if ("context_keywords:".equals(stripped) || "context_keywords :".equals(stripped)) {
                currentSection = "context_keywords"; continue;
            }
            if ("layout_keywords:".equals(stripped) || "layout_keywords :".equals(stripped)) {
                currentSection = "layout_keywords"; continue;
            }

            // --- Inside a section (indented lines) ---
            if (currentSection != null) {
                if (!line.isEmpty() && (line.charAt(0) == ' ' || line.charAt(0) == '\t')) {
                    handleSectionLine(grammar, currentSection, stripped, lineNumber);
                    continue;
                }
                // Non-indented line exits the section
                currentSection = null;
            }

            // --- Token definition: NAME = pattern ---
            int eqIndex = line.indexOf('=');
            if (eqIndex == -1) {
                throw new TokenGrammarError(
                        "Expected token definition (NAME = pattern), got: '" + stripped + "'",
                        lineNumber);
            }

            String namePart = line.substring(0, eqIndex).strip();
            String patternPart = line.substring(eqIndex + 1).strip();

            if (namePart.isEmpty()) {
                throw new TokenGrammarError("Missing token name before '='", lineNumber);
            }
            if (!TOKEN_NAME_PATTERN.matcher(namePart).matches()) {
                throw new TokenGrammarError(
                        "Invalid token name: '" + namePart + "' (must be an identifier)",
                        lineNumber);
            }
            if (patternPart.isEmpty()) {
                throw new TokenGrammarError("Missing pattern after '=' for token '" + namePart + "'", lineNumber);
            }

            grammar.getDefinitions().add(parseDefinition(patternPart, namePart, lineNumber));
        }

        return grammar;
    }

    // --- Section line handler ---

    private static void handleSectionLine(
            TokenGrammar grammar, String section, String stripped, int lineNumber
    ) throws TokenGrammarError {
        if (stripped.isEmpty()) return;

        switch (section) {
            case "keywords":
                grammar.getKeywords().add(stripped);
                break;
            case "reserved":
                grammar.getReservedKeywords().add(stripped);
                break;
            case "context_keywords":
                grammar.getContextKeywords().add(stripped);
                break;
            case "layout_keywords":
                grammar.getLayoutKeywords().add(stripped);
                break;
            case "skip":
                parseAndAddDefinition(grammar.getSkipDefinitions(), stripped, lineNumber, "skip pattern");
                break;
            case "errors":
                parseAndAddDefinition(grammar.getErrorDefinitions(), stripped, lineNumber, "error pattern");
                break;
            default:
                if (section.startsWith("group:")) {
                    String groupName = section.substring(6);
                    parseAndAddGroupDefinition(grammar, groupName, stripped, lineNumber);
                }
                break;
        }
    }

    private static void parseAndAddDefinition(
            List<TokenDefinition> target, String stripped, int lineNumber, String label
    ) throws TokenGrammarError {
        int eqIdx = stripped.indexOf('=');
        if (eqIdx == -1) {
            throw new TokenGrammarError(
                    "Expected " + label + " definition (NAME = pattern), got: '" + stripped + "'",
                    lineNumber);
        }
        String name = stripped.substring(0, eqIdx).strip();
        String pattern = stripped.substring(eqIdx + 1).strip();
        if (name.isEmpty() || pattern.isEmpty()) {
            throw new TokenGrammarError("Incomplete " + label + " definition: '" + stripped + "'", lineNumber);
        }
        target.add(parseDefinition(pattern, name, lineNumber));
    }

    private static void parseAndAddGroupDefinition(
            TokenGrammar grammar, String groupName, String stripped, int lineNumber
    ) throws TokenGrammarError {
        int eqIdx = stripped.indexOf('=');
        if (eqIdx == -1) {
            throw new TokenGrammarError(
                    "Expected token definition in group '" + groupName + "' (NAME = pattern), got: '" + stripped + "'",
                    lineNumber);
        }
        String name = stripped.substring(0, eqIdx).strip();
        String pattern = stripped.substring(eqIdx + 1).strip();
        if (name.isEmpty() || pattern.isEmpty()) {
            throw new TokenGrammarError(
                    "Incomplete definition in group '" + groupName + "': '" + stripped + "'",
                    lineNumber);
        }
        TokenDefinition defn = parseDefinition(pattern, name, lineNumber);

        // Replace the group with one that has the new definition appended
        PatternGroup oldGroup = grammar.getGroups().get(groupName);
        List<TokenDefinition> newDefs = new ArrayList<>(oldGroup.getDefinitions());
        newDefs.add(defn);
        grammar.getGroups().put(groupName, new PatternGroup(groupName, newDefs));
    }

    // --- Definition parser ---

    /**
     * Parse a single token definition pattern with optional -> ALIAS suffix.
     *
     * <p>Supports two forms:
     * <ul>
     *   <li>/regex/ — regex pattern</li>
     *   <li>"literal" — literal string pattern</li>
     * </ul>
     *
     * Either form may have a {@code -> ALIAS} suffix after the closing delimiter.
     */
    static TokenDefinition parseDefinition(String patternPart, String namePart, int lineNumber)
            throws TokenGrammarError {

        if (patternPart.startsWith("/")) {
            // --- Regex pattern ---
            int lastSlash = findClosingSlash(patternPart);
            if (lastSlash == -1) {
                throw new TokenGrammarError("Unclosed regex pattern for token '" + namePart + "'", lineNumber);
            }
            String regexBody = patternPart.substring(1, lastSlash);
            String remainder = patternPart.substring(lastSlash + 1).strip();

            if (regexBody.isEmpty()) {
                throw new TokenGrammarError("Empty regex pattern for token '" + namePart + "'", lineNumber);
            }

            String alias = parseAlias(remainder, namePart, lineNumber);
            return new TokenDefinition(namePart, regexBody, true, lineNumber, alias);

        } else if (patternPart.startsWith("\"")) {
            // --- Literal pattern ---
            int closeQuote = patternPart.indexOf('"', 1);
            if (closeQuote == -1) {
                throw new TokenGrammarError("Unclosed literal pattern for token '" + namePart + "'", lineNumber);
            }
            String literalBody = patternPart.substring(1, closeQuote);
            String remainder = patternPart.substring(closeQuote + 1).strip();

            if (literalBody.isEmpty()) {
                throw new TokenGrammarError("Empty literal pattern for token '" + namePart + "'", lineNumber);
            }

            String alias = parseAlias(remainder, namePart, lineNumber);
            return new TokenDefinition(namePart, literalBody, false, lineNumber, alias);

        } else {
            throw new TokenGrammarError(
                    "Pattern for token '" + namePart + "' must be /regex/ or \"literal\", got: '" + patternPart + "'",
                    lineNumber);
        }
    }

    /** Parse optional -> ALIAS suffix from remainder text after the pattern delimiter. */
    private static String parseAlias(String remainder, String namePart, int lineNumber) throws TokenGrammarError {
        if (remainder.isEmpty()) return null;
        if (remainder.startsWith("->")) {
            String alias = remainder.substring(2).strip();
            if (alias.isEmpty()) {
                throw new TokenGrammarError("Missing alias after '->' for token '" + namePart + "'", lineNumber);
            }
            return alias;
        }
        throw new TokenGrammarError(
                "Unexpected text after pattern for token '" + namePart + "': '" + remainder + "'",
                lineNumber);
    }

    /**
     * Find the closing / in a regex pattern, skipping escaped characters
     * and characters inside [...] character classes.
     *
     * @param s the pattern string starting with /
     * @return index of closing /, or -1 if not found
     */
    static int findClosingSlash(String s) {
        boolean inBracket = false;
        for (int i = 1; i < s.length(); i++) {
            char ch = s.charAt(i);
            if (ch == '\\') {
                i++; // skip escaped character
                continue;
            }
            if (ch == '[' && !inBracket) {
                inBracket = true;
            } else if (ch == ']' && inBracket) {
                inBracket = false;
            } else if (ch == '/' && !inBracket) {
                return i;
            }
        }
        // Fallback: last / as best-effort
        int last = s.lastIndexOf('/');
        return last > 0 ? last : -1;
    }

    /** Strip trailing whitespace (Java 8-compatible helper). */
    private static String stripTrailing(String s) {
        int end = s.length();
        while (end > 0 && Character.isWhitespace(s.charAt(end - 1))) {
            end--;
        }
        return s.substring(0, end);
    }
}
