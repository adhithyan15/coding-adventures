// ============================================================================
// TokenGrammar.java — Complete contents of a parsed .tokens file
// ============================================================================
//
// A .tokens file is the lexical specification for a programming language. It
// describes every token the lexer should recognize, organized into:
//
//   1. Token definitions — NAME = /regex/ or NAME = "literal"
//   2. Keywords section — reserved words promoted from NAME tokens
//   3. Skip section — patterns consumed silently (whitespace, comments)
//   4. Mode directive — special lexer modes like "indentation"
//   5. Pattern groups — context-sensitive token sets
//
// This class holds the parsed result of all those sections.
//
// Layer: TE (text/language layer)
// ============================================================================

package com.codingadventures.grammartools;

import java.util.*;

/**
 * The complete contents of a parsed .tokens file.
 *
 * <p>All fields have sensible defaults so a freshly constructed TokenGrammar
 * represents an empty but valid grammar.
 */
public final class TokenGrammar {

    private int version = 0;
    private boolean caseInsensitive = false;
    private boolean caseSensitive = true;
    private final List<TokenDefinition> definitions = new ArrayList<>();
    private final List<String> keywords = new ArrayList<>();
    private String mode = null;
    private String escapeMode = null;
    private final List<TokenDefinition> skipDefinitions = new ArrayList<>();
    private final List<TokenDefinition> errorDefinitions = new ArrayList<>();
    private final List<String> reservedKeywords = new ArrayList<>();
    private final List<String> contextKeywords = new ArrayList<>();
    private final Map<String, PatternGroup> groups = new LinkedHashMap<>();

    // --- Getters and setters ---

    public int getVersion() { return version; }
    public void setVersion(int version) { this.version = version; }

    public boolean isCaseInsensitive() { return caseInsensitive; }
    public void setCaseInsensitive(boolean v) { this.caseInsensitive = v; }

    public boolean isCaseSensitive() { return caseSensitive; }
    public void setCaseSensitive(boolean v) { this.caseSensitive = v; }

    public List<TokenDefinition> getDefinitions() { return definitions; }
    public List<String> getKeywords() { return keywords; }

    public String getMode() { return mode; }
    public void setMode(String mode) { this.mode = mode; }

    public String getEscapeMode() { return escapeMode; }
    public void setEscapeMode(String escapeMode) { this.escapeMode = escapeMode; }

    public List<TokenDefinition> getSkipDefinitions() { return skipDefinitions; }
    public List<TokenDefinition> getErrorDefinitions() { return errorDefinitions; }
    public List<String> getReservedKeywords() { return reservedKeywords; }
    public List<String> getContextKeywords() { return contextKeywords; }
    public Map<String, PatternGroup> getGroups() { return groups; }

    /**
     * Return the set of all defined token names (including aliases).
     *
     * <p>When a definition has an alias, both the original name and the alias
     * are included. Includes names from all pattern groups.
     *
     * <p>Useful for cross-validation: the parser grammar references tokens by
     * name, and we need to check that every referenced token exists.
     */
    public Set<String> tokenNames() {
        Set<String> names = new HashSet<>();
        List<TokenDefinition> allDefs = new ArrayList<>(definitions);
        for (PatternGroup group : groups.values()) {
            allDefs.addAll(group.getDefinitions());
        }
        for (TokenDefinition d : allDefs) {
            names.add(d.getName());
            if (d.getAlias() != null) {
                names.add(d.getAlias());
            }
        }
        return names;
    }

    /**
     * Return the set of token names as the parser will see them.
     *
     * <p>For definitions with aliases, returns the alias (not the definition
     * name). For definitions without aliases, returns the definition name.
     * Includes names from all pattern groups.
     */
    public Set<String> effectiveTokenNames() {
        Set<String> names = new HashSet<>();
        List<TokenDefinition> allDefs = new ArrayList<>(definitions);
        for (PatternGroup group : groups.values()) {
            allDefs.addAll(group.getDefinitions());
        }
        for (TokenDefinition d : allDefs) {
            names.add(d.getAlias() != null ? d.getAlias() : d.getName());
        }
        return names;
    }
}
