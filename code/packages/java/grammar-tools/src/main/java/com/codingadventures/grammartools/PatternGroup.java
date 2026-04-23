// ============================================================================
// PatternGroup.java — A named set of token definitions for context-sensitive lexing
// ============================================================================
//
// Pattern groups enable context-sensitive lexing. For example, an XML lexer
// defines a "tag" group with patterns for attribute names, equals signs, and
// attribute values. These patterns are only active inside tags — a callback
// pushes the "tag" group when "<" is matched and pops it when ">" is matched.
//
// When a group is at the top of the lexer's group stack, only that group's
// patterns are tried during token matching. Skip patterns remain global.
// ============================================================================

package com.codingadventures.grammartools;

import java.util.Collections;
import java.util.List;
import java.util.Objects;

/**
 * A named set of token definitions that are active together during
 * context-sensitive lexing.
 *
 * @see TokenGrammar
 */
public final class PatternGroup {

    private final String name;
    private final List<TokenDefinition> definitions;

    public PatternGroup(String name, List<TokenDefinition> definitions) {
        this.name = Objects.requireNonNull(name);
        this.definitions = Collections.unmodifiableList(definitions);
    }

    /** The group name, e.g. "tag" or "cdata". */
    public String getName() { return name; }

    /** Ordered list of token definitions. Order matters (first-match-wins). */
    public List<TokenDefinition> getDefinitions() { return definitions; }

    @Override
    public String toString() {
        return "PatternGroup(" + name + ", " + definitions.size() + " definitions)";
    }
}
