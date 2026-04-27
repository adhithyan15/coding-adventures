// ============================================================================
// GrammarElement.java — AST node types for .grammar file EBNF rules
// ============================================================================
//
// A .grammar file uses Extended Backus-Naur Form (EBNF) to describe how tokens
// combine into valid programs. Each grammar rule has a body composed of these
// element types:
//
//   RuleReference — a named reference to another rule or token
//   Literal       — a quoted string literal like "+" or "class"
//   Sequence      — elements that must appear in order (A B C)
//   Alternation   — choices separated by | (A | B | C)
//   Repetition    — zero or more: { element }
//   Optional      — zero or one: [ element ]
//   Group         — parenthesized grouping: ( element )
//   OneOrMoreRepetition  — one or more: { element }+
//   SeparatedRepetition  — zero/one+ with separator: { element // sep }
//   PositiveLookahead    — match without consuming: &element
//   NegativeLookahead    — fail if matches: !element
//
// These types form a sealed hierarchy (via sealed interface) that exhaustive
// pattern matching can check at compile time.
//
// Layer: TE (text/language layer)
// ============================================================================

package com.codingadventures.grammartools;

import java.util.List;

/**
 * Base interface for all grammar rule body elements.
 *
 * <p>Implementations are records for value semantics and pattern matching.
 */
public sealed interface GrammarElement
        permits GrammarElement.RuleReference,
                GrammarElement.Literal,
                GrammarElement.Sequence,
                GrammarElement.Alternation,
                GrammarElement.Repetition,
                GrammarElement.Optional,
                GrammarElement.Group,
                GrammarElement.PositiveLookahead,
                GrammarElement.NegativeLookahead,
                GrammarElement.OneOrMoreRepetition,
                GrammarElement.SeparatedRepetition {

    /** A reference to a rule (lowercase) or token (UPPERCASE). */
    record RuleReference(String name, boolean isToken) implements GrammarElement {}

    /** A quoted string literal in a grammar rule. */
    record Literal(String value) implements GrammarElement {}

    /** Multiple elements that must appear in sequence. */
    record Sequence(List<GrammarElement> elements) implements GrammarElement {}

    /** Alternative choices separated by |. */
    record Alternation(List<GrammarElement> choices) implements GrammarElement {}

    /** Zero or more repetitions: { element }. */
    record Repetition(GrammarElement element) implements GrammarElement {}

    /** Zero or one occurrences: [ element ]. */
    record Optional(GrammarElement element) implements GrammarElement {}

    /** Parenthesized grouping: ( element ). */
    record Group(GrammarElement element) implements GrammarElement {}

    /** Positive lookahead: &element — succeeds without consuming input. */
    record PositiveLookahead(GrammarElement element) implements GrammarElement {}

    /** Negative lookahead: !element — succeeds if element does NOT match. */
    record NegativeLookahead(GrammarElement element) implements GrammarElement {}

    /** One or more repetitions: { element }+. */
    record OneOrMoreRepetition(GrammarElement element) implements GrammarElement {}

    /** Separated repetition: { element // separator } or { element // separator }+. */
    record SeparatedRepetition(GrammarElement element, GrammarElement separator, boolean atLeastOne) implements GrammarElement {}
}
