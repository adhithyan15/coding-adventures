// ============================================================================
// GrammarRule.java — A single rule from a .grammar file
// ============================================================================

package com.codingadventures.grammartools;

/**
 * A named rule from a .grammar file: {@code name = body ;}.
 *
 * @param name       the rule name (lowercase for rules, UPPERCASE for tokens)
 * @param body       the parsed EBNF body
 * @param lineNumber 1-based line number where the rule was defined
 */
public record GrammarRule(String name, GrammarElement body, int lineNumber) {}
